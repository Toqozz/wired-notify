use std::time::Duration;
use std::collections::HashMap;

use winit::{
    event_loop::EventLoopWindowTarget,
    window::WindowId,
};

use crate::{
    rendering::window::NotifyWindow,
    rendering::layout::{LayoutElement, LayoutBlock},
    notification::Notification,
    bus::dbus::DBusNotification,
    maths::Rect,
    config::Config,
};

pub struct NotifyWindowManager {
    //pub windows: Vec<NotifyWindow<'config>>,
    pub monitor_windows: HashMap<u32, Vec<NotifyWindow>>,
    pub dirty: bool,
}

impl NotifyWindowManager {
    pub fn new() -> Self {
        let monitor_windows = HashMap::new();

        Self {
            monitor_windows,
            dirty: false,
        }
    }

    // Summon a new notification.
    pub fn new_notification(&mut self, dbus_notification: DBusNotification, el: &EventLoopWindowTarget<()>) {
        let notification = Notification::from_dbus(dbus_notification);

        if let LayoutElement::NotificationBlock(p) = &Config::get().layout.params {
            self.monitor_windows
                .entry(p.monitor)
                .or_insert(vec![])
                .push(NotifyWindow::new(el, notification));

            // Outer state is now out of sync with internal state because we have an invisible notification.
            self.dirty = true;
        }
    }

    pub fn update(&mut self, delta_time: Duration) {
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
                windows.retain(|w| !w.marked_for_destroy);
            }
        }
    }

    fn update_positions(&mut self) {
        let cfg = Config::get();
        // TODO: gotta do something about this... can't I just cast it?
        if let LayoutElement::NotificationBlock(p) = &cfg.layout.params {
            let gap = &p.gap;
            //let monitor = self.config.monitor.as_ref().expect("No monitor defined.");

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

                let mut prev_pos = LayoutBlock::find_anchor_pos(
                    &cfg.layout.hook,
                    &cfg.layout.offset,
                    &monitor_rect,
                    &Rect::new(0.0, 0.0, 0.0, 0.0)
                );
                prev_pos.x -= gap.x;
                prev_pos.y -= gap.y;

                for i in 0..windows.len() {
                    let window = &windows[i];

                    // Windows which are marked for destroy should be overlapped so that destroying them
                    // will be less noticeable.
                    if window.marked_for_destroy {
                        continue;
                    }

                    // Warning: `set_position` doesn't happen instantly.  If we read
                    // `window_rect`s position straight after this call it probably won't be correct.
                    window.set_position(prev_pos.x + gap.x, prev_pos.y + gap.y);
                    window.set_visible(true);

                    let mut window_rect = window.get_inner_rect();
                    window_rect.set_xy(prev_pos.x + gap.x, prev_pos.y + gap.y);
                    prev_pos = p.notification_hook.get_pos(&window_rect);
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

    // @TODO: there's gotta be a better looking way to do this.
    pub fn drop_window(&mut self, window_id: WindowId) {
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                if window.winit.id() == window_id {
                    window.marked_for_destroy = true;
                    self.dirty = true;

                    break;
                }
            }
        }
    }

    // This feels heavy.
    // Draw an individual window (mostly for expose events).
    pub fn draw_window(&mut self, window_id: WindowId) {
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                if window.winit.id() == window_id {
                    window.draw();
                    break;
                }
            }
        }
    }

    // Draw all windows.
    pub fn _draw_windows(&mut self) {
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                window.draw();
            }
        }
    }
}

