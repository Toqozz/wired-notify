use std::time::Duration;
use std::collections::HashMap;

use dbus::message::SignalArgs;
use dbus::strings::Path;
use winit::{
    dpi::PhysicalPosition,
    event_loop::EventLoopWindowTarget,
    window::WindowId,
    event::ElementState,
    event::MouseButton,
    event::WindowEvent,
    event,
};

use crate::{
    rendering::window::{NotifyWindow, UpdateModes},
    rendering::layout::{LayoutElement, LayoutBlock},
    //notification::Notification,
    bus::self,
    bus::dbus::Notification,
    bus::dbus_codegen::{OrgFreedesktopNotificationsActionInvoked, OrgFreedesktopNotificationsNotificationClosed},
    maths_utility::Rect,
    config::Config,
};

pub struct NotifyWindowManager {
    //pub windows: Vec<NotifyWindow<'config>>,
    pub base_window: winit::window::Window,
    pub monitor_windows: HashMap<u32, Vec<NotifyWindow>>,
    pub dirty: bool,
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
            dirty: false,
        }
    }

    // Summon a new notification.
    pub fn new_notification(&mut self, notification: Notification, el: &EventLoopWindowTarget<()>) {
        if Config::get().debug { dbg!(&notification); }
        if let LayoutElement::NotificationBlock(p) = &Config::get().layout.as_ref().unwrap().params {
            let window = NotifyWindow::new(el, notification, &self);

            let windows = self.monitor_windows
                .entry(p.monitor)
                .or_insert(vec![]);

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
        if Config::get().debug { dbg!(&new_notification); }
        let maybe_window =
            self.monitor_windows
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
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows {
                self.dirty |= window.update(delta_time);
            }
        }

        if self.dirty {
            self.update_positions();
            // Finally drop windows.
            for (_monitor_id, windows) in &mut self.monitor_windows {
                // Send signal for notifications that are going to be closed, then drop them.
                for window in windows.iter().filter(|w| w.marked_for_destroy) {
                    let message = OrgFreedesktopNotificationsNotificationClosed {
                        id: window.notification.id,
                        reason: 4,  // TODO: get real reason. -- 1 expired, 2 dismissed by user, 3 `CloseNotification`, 4 undefined.
                    };
                    let path = Path::new(bus::dbus::PATH).expect("Failed to create DBus path.");
                    let _result = bus::dbus::get_connection().send(message.to_emit_message(&path));
                }
                windows.retain(|w| !w.marked_for_destroy);
            }
        }
    }

    fn update_positions(&mut self) {
        let cfg = Config::get();
        // TODO: gotta do something about this... can't I just cast it?
        if let LayoutElement::NotificationBlock(p) = &cfg.layout.as_ref().unwrap().params {
            for (monitor_id, windows) in &self.monitor_windows {
                // If there are no windows for this monitor, leave it alone.
                if windows.len() == 0 {
                    continue;
                }

                // Grab a winit window reference to use to call winit functions.
                // The functions here are just convenience functions to avoid needing a reference
                // to the event loop -- they don't relate to the individual window.
                let winit_utility = &windows[0].winit;
                let monitor = winit_utility
                    .available_monitors()
                    .nth(*monitor_id as usize)
                    .unwrap_or(winit_utility.primary_monitor());

                let (pos, size) = (monitor.position(), monitor.size());
                let monitor_rect = Rect::new(pos.x.into(), pos.y.into(), size.width.into(), size.height.into());
                let mut prev_rect = monitor_rect;

                let mut real_idx = 0;
                for i in 0..windows.len() {
                    // Windows which are marked for destroy should be overlapped so that destroying them
                    // will be less noticeable.
                    if windows[i].marked_for_destroy {
                        continue;
                    }

                    let window = &windows[i];
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
                        LayoutBlock::find_anchor_pos(
                            &p.notification_hook,
                            &p.gap,
                            &prev_rect,
                            &window_rect,
                        )
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
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                match button {
                    MouseButton::Left => pressed = Some(1),
                    MouseButton::Right => pressed = Some(2),
                    MouseButton::Middle => pressed = Some(3),
                    MouseButton::Other(u) => pressed = Some(u),
                };
            },

            WindowEvent::CursorMoved { position, .. } => {
                self.find_window_mut(window_id).unwrap().process_mouse_move(position);
            },

            // If we don't notify when the cursor left, then we have issues moving the cursor off
            // the side of the window and the hover status not being reset.
            // Since the position given is from the top left of the window, a negative value should
            // always be outside it.
            WindowEvent::CursorLeft { .. } => {
                self.find_window_mut(window_id).unwrap().process_mouse_move(PhysicalPosition::new(-1.0, -1.0));
            }

            _ => (),
        }

        // If nothing was pressed, then there is no event to process.
        // The code below won't work with None naturally, because the config is allowed to have
        // None shortcuts.
        if !pressed.is_some() {
            return;
        }

        let config = Config::get();
        if pressed == config.shortcuts.notification_interact {
            self.find_window_mut(window_id).unwrap().process_mouse_click();
        } else if pressed == config.shortcuts.notification_close {
            self.drop_window(window_id);
        } else if pressed == config.shortcuts.notification_closeall {
            self.drop_windows();
        }  else if pressed == config.shortcuts.notification_pause {
            if let Some((monitor, idx)) = self.find_window_idx(window_id) {
                let window = self.monitor_windows
                    .get_mut(&monitor).unwrap()
                    .get_mut(idx).unwrap();

                window.update_mode.toggle(UpdateModes::FUSE);
            }
        } else {
            let notification = &self.find_window(window_id).unwrap().notification;
            // Creates an iterator without the "default" key, which is preserved for action1.
            let mut keys = notification.actions.keys().filter(|s| *s != "default");

            // action1 is the default action.  Maybe we should rename it to action_default or
            // something.
            let key = if pressed == config.shortcuts.notification_action1 {
                if notification.actions.contains_key("default") {
                    Some("default".to_owned())
                } else {
                    None
                }
            } else if pressed == config.shortcuts.notification_action2 {
                keys.nth(0).cloned()
            } else if pressed == config.shortcuts.notification_action3 {
                keys.nth(1).cloned()
            } else if pressed == config.shortcuts.notification_action4 {
                keys.nth(2).cloned()
            } else {
                None
            };

            // Found an action -> button press combo, great!  Send dbus a signal to invoke it.
            if let Some(k) = key {
                let message = OrgFreedesktopNotificationsActionInvoked {
                    action_key: k.to_owned(), id: notification.id
                };
                let path = Path::new(bus::dbus::PATH).expect("Failed to create DBus path.");
                let _result = bus::dbus::get_connection().send(message.to_emit_message(&path));
            } else {
                println!("Received action shortcut but could not find a matching action to trigger.");
            }
        }
    }

    // Find window across all monitors based on an id, which we receive from winit events.
    pub fn find_window_idx(&self, window_id: WindowId) -> Option<(u32, usize)> {
        for (monitor, windows) in &self.monitor_windows {
            let found = windows.iter().position(|w| w.winit.id() == window_id);
            if let Some(idx) = found {
                return Some((*monitor, idx))
            }
        }

        None
    }

    // This call should always succeed, unless for some reason the event came from a window that we
    // don't know about -- which should never happen, or the id is wrong.
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

    pub fn notification_exists(&self, id: u32) -> bool {
        for m in self.monitor_windows.values() {
            for w in m {
                if w.notification.id == id {
                    return true;
                }
            }
        }

        return false;
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            let window = self.monitor_windows
                .get_mut(&monitor).unwrap()
                .get_mut(idx).unwrap();

            window.marked_for_destroy = true;
            self.dirty = true;
        }
    }

    // @TODO: how about a shortcut for dropping all windows on one monitor?
    pub fn drop_windows(&mut self) {
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                window.marked_for_destroy = true;
                self.dirty = true;
            }
        }
    }

    // Draw an individual window (mostly for expose events).
    pub fn request_redraw(&mut self, window_id: WindowId) {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            let window = self.monitor_windows
                .get_mut(&monitor).unwrap()
                .get_mut(idx).unwrap();

            window.dirty = true;
        }
    }

    pub fn drop_notification(&mut self, id: u32) {
        // This should be Some, otherwise we were given a bad id.
        let maybe_window = self.monitor_windows.values_mut().flatten().find(|w| w.notification.id == id);
        if let Some(window) = maybe_window {
            window.marked_for_destroy = true;
        }
    }
}
