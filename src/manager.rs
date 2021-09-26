use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use dbus::message::SignalArgs;
use dbus::strings::Path;
use winit::{
    dpi::PhysicalPosition, event, event::ElementState, event::MouseButton, event::WindowEvent,
    event_loop::EventLoopWindowTarget, window::WindowId,
};

use crate::{
    //notification::Notification,
    bus,
    bus::dbus::Notification,
    bus::dbus_codegen::{
        OrgFreedesktopNotificationsActionInvoked, OrgFreedesktopNotificationsNotificationClosed,
    },
    config::Config,
    maths_utility::{self, Rect},
    rendering::layout::{LayoutBlock, LayoutElement},
    rendering::window::{NotifyWindow, UpdateModes},
};

pub struct NotifyWindowManager {
    pub base_window: winit::window::Window,
    pub monitor_windows: HashMap<u32, Vec<NotifyWindow>>,
    pub history: VecDeque<Notification>,
    pub dirty: bool,

    idle_check_timer: f32,
}

impl NotifyWindowManager {
    pub fn new(el: &EventLoopWindowTarget<()>) -> Self {
        let monitor_windows = HashMap::new();

        let base_window = winit::window::WindowBuilder::new()
            .with_visible(false)
            .build(el)
            .expect("Failed to create base window.");

        Self {
            base_window,
            monitor_windows,
            history: VecDeque::with_capacity(Config::get().history_length),
            dirty: false,

            idle_check_timer: 0.0,
        }
    }

    // Summon a new notification.
    pub fn new_notification(&mut self, notification: Notification, el: &EventLoopWindowTarget<()>) {
        if Config::get().debug {
            dbg!(&notification);
        }

        if let LayoutElement::NotificationBlock(p) = &Config::get().layout.as_ref().unwrap().params {
            let window = NotifyWindow::new(el, notification, self);

            let windows = self.monitor_windows.entry(p.monitor).or_insert_with(Vec::new);

            // Push a new notification window.
            windows.push(window);

            // If we've exceeded max notifications, then mark the top-most one for destroy.
            let cfg = Config::get();
            if cfg.max_notifications > 0 && windows.len() > cfg.max_notifications {
                windows.first_mut().unwrap().marked_for_destroy = true;
            }

            // Outer state is now out of sync with internal state because we have an invisible notification.
            self.dirty = true;
        }
    }

    // This function assumes that there is a notification to replace, otherwise it does nothing.
    pub fn replace_notification(&mut self, new_notification: Notification) {
        if Config::get().debug {
            dbg!(&new_notification);
        }
        let maybe_window = self
            .monitor_windows
            .values_mut()
            .flatten()
            .find(|w| w.notification.id == new_notification.id);

        // It may be that the notification has already expired, in which case we just ignore the
        // update request.
        if let Some(window) = maybe_window {
            // Replacing notification data may mean the notification position has to change.
            window.replace_notification(new_notification);
        }
    }

    pub fn update(&mut self, delta_time: Duration) {
        // Update windows and then check for dirty state.
        // Returning dirty from a window update means the window has been deleted / needs
        // positioning updated.
        for window in &mut self.monitor_windows.values_mut().flatten() {
            self.dirty |= window.update(delta_time);
        }

        if self.dirty {
            self.update_positions();
            // Finally drop windows.
            #[allow(clippy::for_kv_map)]    // Better that we keep the monitor in mind here for future.
            for (_monitor_id, windows) in &mut self.monitor_windows {
                // Send signal for notifications that are going to be closed, then drop them.
                for window in windows.iter().filter(|w| w.marked_for_destroy) {
                    let message = OrgFreedesktopNotificationsNotificationClosed {
                        id: window.notification.id,
                        reason: 4, // TODO: get real reason. -- 1 expired, 2 dismissed by user, 3 `CloseNotification`, 4 undefined.
                    };
                    let path = Path::new(bus::dbus::PATH).expect("Failed to create DBus path.");
                    let _result = bus::dbus::get_connection().send(message.to_emit_message(&path));

                    // Window, dying push notification to history.
                    // NOTE: if window dies in some other way (which we don't support), we won't
                    // have a history of it.  I think this might cause a bug anyway, since the
                    // window would still be in the array here.
                    // A fix may be to write notifications to history as soon as we receive them,
                    // but then we need to keep track of which notifications are active and stuff.
                    if self.history.len() + 1 > Config::get().history_length {
                        self.history.pop_front();
                    }
                    self.history.push_back(window.notification.clone());
                }
                windows.retain(|w| !w.marked_for_destroy);
            }
        }

        // TODO: SHOULD_CHECK_IDLE? Somewhere?
        // TODO: there should probably be a way to make this so we only need to set new windows  
        // update mode instead of just brute forcing them.  but maybe this is fine anyway?
        // Definitely less bug prone.
        // Our idle threshold granularity is 1s, so we can save time by only checking at that
        // frequency.
        self.idle_check_timer += delta_time.as_secs_f32();
        if self.idle_check_timer > 1.0 {
            self.idle_check_timer = 0.0;

            if let Some(threshold) = Config::get().idle_threshold {
                match maths_utility::query_screensaver_info(&self.base_window) {
                    Ok(info) => {
                        if info.idle / 1000 >= threshold {
                            self.monitor_windows
                                .values_mut()
                                .flatten()
                                .for_each(|w| w.update_mode = UpdateModes::DRAW);
                        }
                    },
                    Err(e) => eprintln!("{}", e),
                }
            }
        }
    }

    fn update_positions(&mut self) {
        let cfg = Config::get();
        // TODO: gotta do something about this... can't I just cast it?
        if let LayoutElement::NotificationBlock(p) = &cfg.layout.as_ref().unwrap().params {
            for (monitor_id, windows) in &self.monitor_windows {
                // If there are no windows for this monitor, leave it alone.
                if windows.is_empty() {
                    continue;
                }

                // Grab a winit window reference to use to call winit functions.
                // The functions here are just convenience functions to avoid needing a reference
                // to the event loop -- they don't relate to the individual window.
                let winit_utility = &windows[0].winit;
                let maybe_monitor = winit_utility
                    .available_monitors()
                    .nth(*monitor_id as usize)
                    .or_else(|| winit_utility.primary_monitor());

                // If we can't find a monitor, it's basically over.
                // But let's not crash.  Maybe we'll find a monitor next time (if it was unplugged
                // or something).
                let monitor = match maybe_monitor {
                    Some(m) => m,
                    None => continue,
                };

                let (pos, size) = (monitor.position(), monitor.size());
                let monitor_rect =
                    Rect::new(pos.x.into(), pos.y.into(), size.width.into(), size.height.into());
                let mut prev_rect = monitor_rect;

                let mut real_idx = 0;
                for window in windows {
                    // Windows which are marked for destroy should be overlapped so that destroying them
                    // will be less noticeable.
                    if window.marked_for_destroy {
                        continue;
                    }

                    let mut window_rect = window.get_inner_rect();

                    // For the first notification, we attach to the monitor.
                    // For the second and more notifications, we attach to the previous
                    // notification.
                    let pos = if real_idx == 0 {
                        LayoutBlock::find_anchor_pos(
                            &cfg.layout.as_ref().unwrap().hook,
                            &cfg.layout.as_ref().unwrap().offset,
                            &prev_rect,
                            &window_rect,
                        )
                    } else {
                        LayoutBlock::find_anchor_pos(&p.notification_hook, &p.gap, &prev_rect, &window_rect)
                    };

                    // Note: `set_position` doesn't happen instantly.  If we read
                    // `get_rect()`s position straight after this call it probably won't be correct,
                    // which is why we `set_xy` manually after.
                    window.set_position(pos.x, pos.y);
                    window.set_visible(true);

                    window_rect.set_xy(pos.x, pos.y);
                    prev_rect = window_rect;

                    real_idx += 1;
                }
            }
        } else {
            // Panic because the config must have not been setup properly.
            // @TODO: move this panic to config verify, and then just unwrap or something instead.
            panic!("The root LayoutElement must be a NotificationBlock!");
        }

        // Outer state is now up to date with internal state.
        self.dirty = false;
    }

    pub fn process_event(&mut self, window_id: WindowId, event: event::WindowEvent) {
        let mut pressed = None;
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                // We need to handle the Option here (and below) properly, because events can come in any order
                // and the move/click/shortcut event is not guaranteed to come before the window is
                // destroyed.
                if let Some(window) = self.find_window_mut(window_id) {
                    window.process_mouse_move(position);
                }
            }

            // If we don't notify when the cursor left, then we have issues moving the cursor off
            // the side of the window and the hover status not being reset.
            // Since the position given is from the top left of the window, a negative value should
            // always be outside it.
            WindowEvent::CursorLeft { .. } => {
                if let Some(window) = self.find_window_mut(window_id) {
                    window.process_mouse_move(PhysicalPosition::new(-1.0, -1.0));
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                match button {
                    MouseButton::Left => pressed = Some(1),
                    MouseButton::Right => pressed = Some(2),
                    MouseButton::Middle => pressed = Some(3),
                    MouseButton::Other(u) => pressed = Some(u),
                };
            }

            _ => (),
        }

        // If nothing was pressed, then there is no event to process.
        // The code below won't work with None naturally, because the config is allowed to have
        // None shortcuts.
        if pressed.is_none() {
            return;
        }

        let config = Config::get();
        if pressed == config.shortcuts.notification_interact {
            if let Some(window) = self.find_window_mut(window_id) {
                window.process_mouse_click();
            }
        } else if pressed == config.shortcuts.notification_closeall {
            self.drop_windows();
        } else if pressed == config.shortcuts.notification_pause {
            if let Some((monitor, idx)) = self.find_window_idx(window_id) {
                let window = self.monitor_windows
                    .get_mut(&monitor).unwrap()
                    .get_mut(idx).unwrap();

                window.update_mode.toggle(UpdateModes::FUSE);
            }
        } else {
            // Request the window to be dropped if we got a close action.
            if [
                config.shortcuts.notification_close,
                config.shortcuts.notification_action1_and_close,
                config.shortcuts.notification_action2_and_close,
                config.shortcuts.notification_action3_and_close,
                config.shortcuts.notification_action4_and_close,
            ]
            .contains(&pressed)
            {
                self.drop_window_id(window_id);
            }

            let action_id = if pressed == config.shortcuts.notification_action1
                || pressed == config.shortcuts.notification_action1_and_close {
                0
            } else if pressed == config.shortcuts.notification_action2
                || pressed == config.shortcuts.notification_action2_and_close {
                1
            } else if pressed == config.shortcuts.notification_action3
                || pressed == config.shortcuts.notification_action3_and_close {
                2
            } else if pressed == config.shortcuts.notification_action4
                || pressed == config.shortcuts.notification_action4_and_close {
                3
            } else {
                // `pressed` did not match any action key.
                return;
            };

            self.trigger_action_idx(window_id, action_id);
        }
    }

    pub fn trigger_action_idx(&mut self, window_id: WindowId, action: usize) {
        let notification = match self.find_window(window_id) {
            Some(w) => &w.notification,
            None => return,
        };

        let mut keys = notification.actions.keys().filter(|s| *s != "default");
        let key = if action == 0 {
            if notification.actions.contains_key("default") {
                Some("default".to_owned())
            } else {
                None
            }
        } else {
            keys.nth(action - 1).cloned()
        };

        if let Some(k) = key {
            let message = OrgFreedesktopNotificationsActionInvoked {
                action_key: k,
                id: notification.id,
            };
            let path = Path::new(bus::dbus::PATH).expect("Failed to create DBus path.");
            let _result = bus::dbus::get_connection().send(message.to_emit_message(&path));
        } else {
            eprintln!("Tried to trigger an action with id: {}, but couldn't find any matches.", action);
        }
    }

    // Find window across all monitors based on an id, which we receive from winit events.
    pub fn find_window_idx(&self, window_id: WindowId) -> Option<(u32, usize)> {
        for (monitor, windows) in &self.monitor_windows {
            let found = windows.iter().position(|w| w.winit.id() == window_id);
            if let Some(idx) = found {
                return Some((*monitor, idx));
            }
        }

        None
    }

    // This call should always succeed, unless for some reason the event came from a window that we
    // don't know about -- which should never happen, or the id is wrong.
    // Potentially, this call is delayed and we've already dropped that window, which is definitely
    // plausible.
    pub fn find_window(&self, window_id: WindowId) -> Option<&NotifyWindow> {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            if let Some(windows) = self.monitor_windows.get(&monitor) {
                return windows.get(idx);
            }
        }

        None
    }

    pub fn find_window_mut(&mut self, window_id: WindowId) -> Option<&mut NotifyWindow> {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            if let Some(windows) = self.monitor_windows.get_mut(&monitor) {
                return windows.get_mut(idx);
            }
        }

        None
    }

    pub fn find_window_ordered(&self, num: usize) -> Option<WindowId> {
        let mut windows: Vec<&NotifyWindow> =
            self.monitor_windows.values()
                                .flatten()
                                .collect();

        // `sort_unstable` is faster, but windows with the exact same creation timestamp may by
        // shifted in ordering, which is undersireable.  DateTime is probably precise enough to get
        // away with this, but frankly I just don't want to worry about it.
        windows.sort_by(|a, b| a.creation_timestamp.partial_cmp(&b.creation_timestamp).unwrap());
        windows.get(num).map(|w| w.winit.id())
    }

    pub fn find_window_nid(&self, notification_id: u32) -> Option<WindowId> {
        self.monitor_windows
            .values()
            .flatten()
            .find(|w| w.notification.id == notification_id)
            .map(|w| w.winit.id())
    }

    pub fn notification_exists(&self, id: u32) -> bool {
        for m in self.monitor_windows.values() {
            for w in m {
                if w.notification.id == id {
                    return true;
                }
            }
        }

        false
    }

    // Drop a window.  Return true if we found the window and told it to drop, false otherwise.
    pub fn drop_window_id(&mut self, window_id: WindowId) -> bool {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            let window = self.monitor_windows
                .get_mut(&monitor).unwrap()
                .get_mut(idx).unwrap();

            window.marked_for_destroy = true;
            self.dirty = true;
            return true;
        }

        false
    }

    // @TODO: how about a shortcut for dropping all windows on one monitor?  Support multi-monitor
    // better first.
    pub fn drop_windows(&mut self) {
        #[allow(clippy::for_kv_map)]
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                window.marked_for_destroy = true;
                self.dirty = true;
            }
        }
    }

    // Draw an individual window (mostly for expose events).
    pub fn request_redraw(&mut self, window_id: WindowId) -> bool {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            let window = self.monitor_windows
                .get_mut(&monitor).unwrap()
                .get_mut(idx).unwrap();

            window.dirty = true;
            return true;
        }

        false
    }

    pub fn drop_notification(&mut self, id: u32) -> bool {
        // This should be Some, otherwise we were given a bad id.
        let maybe_window = self.monitor_windows.values_mut().flatten().find(|w| w.notification.id == id);
        if let Some(window) = maybe_window {
            window.marked_for_destroy = true;
            self.dirty = true;
            return true;
        }

        false
    }
}
