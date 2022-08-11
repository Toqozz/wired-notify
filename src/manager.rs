use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use dbus::channel::Sender;
use dbus::message::SignalArgs;
use dbus::strings::Path;
use winit::{
    dpi::PhysicalPosition, event, event::ElementState, event::MouseButton, event::WindowEvent,
    event_loop::EventLoopWindowTarget, monitor::MonitorHandle, window::WindowId,
};

use crate::config::FollowMode;
use crate::{
    //notification::Notification,
    bus,
    bus::dbus::Notification,
    bus::dbus_codegen::{
        OrgFreedesktopNotificationsActionInvoked, OrgFreedesktopNotificationsNotificationClosed,
    },
    config::Config,
    maths_utility::{self, Rect},
    rendering::layout::LayoutBlock,
    rendering::window::{NotifyWindow, UpdateModes},
};

pub struct NotifyWindowManager {
    pub base_window: winit::window::Window,
    pub layout_windows: HashMap<String, Vec<NotifyWindow>>,
    pub history: VecDeque<Notification>,
    pub dirty: bool,

    // Do not disturb.
    dnd: bool,
    // For "expensive" updates that don't have to happen every frame.
    slow_update_timer: f32,
    // The idle timer last frame, from xss.
    last_idle_time: u64,
    active_monitor: Option<MonitorHandle>,
}

impl NotifyWindowManager {
    pub fn new(el: &EventLoopWindowTarget<()>) -> Self {
        // Create a map for each layout type, which allows us to easily keep track of different
        // layouts later.
        let mut layout_windows = HashMap::new();
        for layout in &Config::get().layouts {
            layout_windows.insert(layout.name.to_owned(), vec![]);
        }

        let base_window = winit::window::WindowBuilder::new()
            .with_visible(false)
            .build(el)
            .expect("Failed to create base window.");

        let active_monitor = match Config::get().focus_follows {
            FollowMode::Mouse => maths_utility::get_active_monitor_mouse(&base_window),
            FollowMode::Window => maths_utility::get_active_monitor_keyboard(&base_window),
        };

        Self {
            base_window,
            layout_windows,
            history: VecDeque::with_capacity(Config::get().history_length),
            dirty: false,

            dnd: false,
            slow_update_timer: 0.0,
            last_idle_time: 0,
            active_monitor,
        }
    }

    // Summon a new notification.
    pub fn new_notification(&mut self, notification: Notification, el: &EventLoopWindowTarget<()>) {
        for layout in &Config::get().layouts {
            // Spawn a new window for each "root" layout that should be drawn.
            // If this layout doesn't meet any criteria, skip, obviously.
            if !notification_meets_layout_criteria(layout, &notification) {
                continue;
            } else {
                let window = NotifyWindow::new(el, notification.clone(), layout.clone(), self);

                // Find this notification's layout and push the window there.
                let windows = self
                    .layout_windows
                    .get_mut(&layout.name)
                    .expect("Somehow created a new layout.");
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
    }

    pub fn replace_or_spawn(&mut self, notification: Notification, el: &EventLoopWindowTarget<()>) {
        let cfg = Config::get();
        if cfg.debug {
            dbg!(self.dnd, &notification);
        }

        // Right now we just book it if dnd is enabled.
        if self.dnd {
            return;
        }

        // Find any windows that have the same id, or the same app name and tag.
        // If one exists then we should replace that (if replacing is enabled).
        let mut maybe_windows = vec![];
        //let mut maybe_window = None;
        for w in self.layout_windows.values_mut().flatten() {
            if notification_meets_layout_criteria(w.layout.as_ref().unwrap(), &notification)
                && ((w.notification.id == notification.id && cfg.replacing_enabled)
                    || (w.notification.app_name == notification.app_name
                        && w.notification.tag.is_some()
                        && w.notification.tag == notification.tag))
            {
                maybe_windows.push(w);
                //maybe_window = Some(w)
            }
        }

        if maybe_windows.len() > 0 {
            for w in maybe_windows {
                w.replace_notification(notification.clone());
            }
        } else {
            self.new_notification(notification, el);
        }
    }

    pub fn update(&mut self, delta_time: Duration) {
        // Idle threshold granularity is 1s, but I want to update active monitor faster than
        // that.
        self.slow_update_timer += delta_time.as_secs_f32();
        if self.slow_update_timer > 0.33 {
            self.slow_update_timer = 0.0;

            // Could probably only update this if we're actually configured to follow a monitor,
            // but it's not really worth it.
            let active_monitor = match Config::get().focus_follows {
                FollowMode::Mouse => maths_utility::get_active_monitor_mouse(&self.base_window),
                FollowMode::Window => maths_utility::get_active_monitor_keyboard(&self.base_window),
            };
            if active_monitor != self.active_monitor {
                self.active_monitor = active_monitor;
                self.dirty = true;
            }

            let cfg = Config::get();
            if let Some(threshold) = cfg.idle_threshold {
                match maths_utility::query_screensaver_info(&self.base_window) {
                    Ok(info) => {
                        // 1s to be considered idle.
                        let idle_last_frame = self.is_idle_1s();
                        let is_idle = info.idle / 1000 >= 1;

                        // If we "woke up" this frame.
                        if cfg.unpause_on_input && idle_last_frame && !is_idle {
                            self.layout_windows
                                .values_mut()
                                .flatten()
                                .for_each(|w| w.update_mode = UpdateModes::all());
                        }

                        // Just pause them every "frame", it's ok.
                        if info.idle / 1000 >= threshold {
                            self.layout_windows
                                .values_mut()
                                .flatten()
                                .for_each(|w| w.update_mode = UpdateModes::DRAW);
                        }

                        self.last_idle_time = info.idle;
                    }
                    Err(e) => eprintln!("{}", e),
                }
            }
        }

        // Update windows and then check for dirty state.
        // Returning dirty from a window update means the window has been deleted / needs
        // positioning updated.
        for window in &mut self.layout_windows.values_mut().flatten() {
            self.dirty |= window.update(delta_time);
        }

        if self.dirty {
            self.update_positions();
            // Finally drop windows.
            for windows in self.layout_windows.values_mut() {
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
    }

    fn update_positions(&mut self) {
        let cfg = Config::get();
        for (layout_name, windows) in &self.layout_windows {
            // If there are no windows for this layout, leave it alone.
            if windows.is_empty() {
                continue;
            }

            // Grab the layout associated with these windows.
            let layout = cfg
                .layouts
                .iter()
                .find(|l| &l.name == layout_name)
                .expect("Failed to find matching layout.");
            let layout_params = layout.as_notification_block();

            let monitor = {
                let mut maybe_monitor;
                // Use cursor focus.
                if layout_params.monitor < 0 {
                    maybe_monitor = self.active_monitor.clone();
                } else {
                    maybe_monitor = self
                        .base_window
                        .available_monitors()
                        .nth(layout_params.monitor as usize)
                }

                // Fallback, try to use primary monitor.
                if maybe_monitor.is_none() {
                    maybe_monitor = self.base_window.primary_monitor();
                }

                // If we can't find a monitor, it's basically over.
                // But we don't have to crash.  Maybe we'll find a monitor next
                // time (if it was unplugged or something).
                match maybe_monitor {
                    Some(m) => m,
                    None => continue,
                }
            };

            let (pos, size) = (monitor.position(), monitor.size());
            let monitor_rect = Rect::new(pos.x.into(), pos.y.into(), size.width.into(), size.height.into());
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
                    LayoutBlock::find_anchor_pos(&layout.hook, &layout.offset, &prev_rect, &window_rect)
                } else {
                    LayoutBlock::find_anchor_pos(
                        &layout_params.notification_hook,
                        &layout_params.gap,
                        &prev_rect,
                        &window_rect,
                    )
                };

                // Note: `set_position` doesn't happen instantly.  If we read
                // `get_rect()`s position straight after this call it probably won't be correct,
                // which is why we `set_xy` manually after.
                window.set_position(pos.x, pos.y);
                window_rect.set_xy(pos.x, pos.y);
                prev_rect = window_rect;

                real_idx += 1;
            }
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
            if let Some(window) = self.find_window_mut(window_id) {
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
                config.shortcuts.notification_interact_and_close,
            ]
            .contains(&pressed)
            {
                self.drop_window_id(window_id);
            }

            let action_id = if pressed == config.shortcuts.notification_action1
                || pressed == config.shortcuts.notification_action1_and_close
            {
                0
            } else if pressed == config.shortcuts.notification_action2
                || pressed == config.shortcuts.notification_action2_and_close
            {
                1
            } else if pressed == config.shortcuts.notification_action3
                || pressed == config.shortcuts.notification_action3_and_close
            {
                2
            } else if pressed == config.shortcuts.notification_action4
                || pressed == config.shortcuts.notification_action4_and_close
            {
                3
            } else if pressed == config.shortcuts.notification_interact
                || pressed == config.shortcuts.notification_interact_and_close
            {
                if let Some(window) = self.find_window_mut(window_id) {
                    window.process_mouse_click();
                }
                return;
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
            let _result = bus::dbus::get_connection()
                .channel()
                .send(message.to_emit_message(&path));
        } else {
            eprintln!(
                "Tried to trigger an action with id: {}, but couldn't find any matches.",
                action
            );
        }
    }

    // Find window across all monitors based on an id, which we receive from winit events.
    pub fn _find_window_idx(&self, window_id: WindowId) -> Option<(&str, usize)> {
        for (layout_name, windows) in &self.layout_windows {
            let found = windows.iter().position(|w| w.winit.id() == window_id);
            if let Some(idx) = found {
                return Some((layout_name, idx));
            }
        }

        None
    }

    // This call should always succeed, unless for some reason the event came from a window that we
    // don't know about -- which should never happen, or the id is wrong.
    // Potentially, this call is delayed and we've already dropped that window, which is definitely
    // plausible.
    pub fn find_window(&self, window_id: WindowId) -> Option<&NotifyWindow> {
        self.layout_windows
            .values()
            .flatten()
            .find(|w| w.winit.id() == window_id)
    }

    pub fn find_window_mut(&mut self, window_id: WindowId) -> Option<&mut NotifyWindow> {
        self.layout_windows
            .values_mut()
            .flatten()
            .find(|w| w.winit.id() == window_id)
    }

    pub fn find_window_ordered(&self, num: usize) -> Option<WindowId> {
        let mut windows: Vec<&NotifyWindow> = self.layout_windows.values().flatten().collect();

        // `sort_unstable` is faster, but windows with the exact same creation timestamp may by
        // shifted in ordering, which is undersireable.  DateTime is probably precise enough to get
        // away with this, but frankly I just don't want to worry about it.
        windows.sort_by(|a, b| a.creation_timestamp.partial_cmp(&b.creation_timestamp).unwrap());
        windows.get(num).map(|w| w.winit.id())
    }

    pub fn find_window_nid(&self, notification_id: u32) -> Option<WindowId> {
        self.layout_windows
            .values()
            .flatten()
            .find(|w| w.notification.id == notification_id)
            .map(|w| w.winit.id())
    }

    // Drop a window.  Return true if we found the window and told it to drop, false otherwise.
    pub fn drop_window_id(&mut self, window_id: WindowId) -> bool {
        if let Some(window) = self.find_window_mut(window_id) {
            window.marked_for_destroy = true;
            self.dirty = true;
            return true;
        }

        false
    }

    // @TODO: how about a shortcut for dropping all windows on one monitor?  Support multi-monitor
    // better first.
    pub fn drop_windows(&mut self) {
        self.layout_windows
            .values_mut()
            .flatten()
            .for_each(|w| w.marked_for_destroy = true);
        self.dirty = true;
        /*
        #[allow(clippy::for_kv_map)]
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                window.marked_for_destroy = true;
                self.dirty = true;
            }
        }
        */
    }

    // Draw an individual window (mostly for expose events).
    pub fn request_redraw(&mut self, window_id: WindowId) -> bool {
        if let Some(window) = self.find_window_mut(window_id) {
            window.dirty = true;
            return true;
        }

        false
    }

    pub fn drop_notification(&mut self, id: u32) -> bool {
        // This should be Some, otherwise we were given a bad id.
        let maybe_window = self
            .layout_windows
            .values_mut()
            .flatten()
            .find(|w| w.notification.id == id);
        if let Some(window) = maybe_window {
            window.marked_for_destroy = true;
            self.dirty = true;
            return true;
        }

        false
    }

    pub fn has_windows(&self) -> bool {
        self.layout_windows.values().any(|m| m.len() > 0)
    }

    pub fn is_idle_1s(&self) -> bool {
        // >=1s considered idle.
        self.last_idle_time / 1000 >= 1
    }

    pub fn is_idle_for(&self, threshold: u64) -> bool {
        self.last_idle_time / 1000 >= threshold
    }

    pub fn set_dnd(&mut self, val: bool) {
        self.dnd = val;
    }
}

// Checks if this block should be drawn (according to render criteria for each block).
// The root block is handled differently.
// If the root block doesn't meet criteria, then don't draw at all.
// If the root block does meet criteria, but no child blocks do, then don't draw.
pub fn notification_meets_layout_criteria(root: &LayoutBlock, notification: &Notification) -> bool {
    // Root block handled differently to be more intuitive.
    if !root.should_draw(notification) {
        return false;
    }

    for child in &root.children {
        if child.should_draw(notification) {
            return true;
        }
    }

    false
}

// Checks if this block should be drawn (according to render criteria for each block).
// The root block is handled differently.
// If the root block doesn't meet criteria, then don't draw at all.
// If the root block does meet criteria, but no child blocks do, then don't draw.
/*
pub fn notification_meets_any_criteria(notification: &Notification) -> bool {
    let layout = Config::get().layout.as_ref().unwrap();

    // Root block handled differently to be more intuitive.
    if !layout.should_draw(notification) {
        return false;
    }

    for child in &layout.children {
        if child.should_draw(notification) {
            return true;
        }
    }

    false
}
*/
