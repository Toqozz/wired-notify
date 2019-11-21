use winit::{
    event_loop::EventLoopWindowTarget,
    window::WindowId,
};

use crate::bus::dbus::DBusNotification;
use crate::config::Config;
use crate::rendering::window::NotifyWindow;
use crate::types::maths::Rect;
use crate::rendering::layout::LayoutBlock;
use std::time::Duration;
use crate::notification::Notification;

pub struct NotifyWindowManager<'config> {
    pub windows: Vec<NotifyWindow<'config>>,
    pub dirty: bool,

    pub config: &'config Config,
}

impl<'config> NotifyWindowManager<'config> {
    pub fn new(config: &'config Config) -> Self {
        //let notify_windows = Vec::new();
        let windows = vec![];

        Self {
            windows,
            dirty: false,
            config,
        }
    }

    // Summon a new notification.
    pub fn new_notification(&mut self, dbus_notification: DBusNotification, el: &EventLoopWindowTarget<()>) {
        let notification = Notification::from_dbus(dbus_notification, self.config);
        dbg!(&notification);

        let mut window = NotifyWindow::new(&self.config, el, notification);
        let (rect, delta) = window.predict_size();
        window.set_size(rect.width(),rect.height());
        window.master_offset = delta;

        self.windows.push(window);

        // Outer state is now out of sync with internal state because we have an invisible notification.
        self.dirty = true;
    }

    pub fn update(&mut self, delta_t: Duration) {
        self.update_timers(delta_t);
        if self.dirty {
            self.update_positions();
            self.windows.retain(|w| !w.marked_for_destroy);
        }
    }

    fn update_timers(&mut self, time_passed: Duration) {
        for window in &mut self.windows {
            window.notification.fuse -= time_passed.as_millis() as i32;
            if window.notification.fuse < 0 {
                // Window will be destroyed after others have been repositioned to replace it.
                window.marked_for_destroy = true;
                self.dirty = true;
            }
        }
    }

    fn update_positions(&mut self) {
        if let LayoutBlock::NotificationBlock(p) = &self.config.layout {
            let gap = &p.gap;
            let monitor = self.config.monitor.as_ref().expect("No monitor defined.");

            let (pos, size) = (monitor.position(), monitor.size());
            let monitor_rect = Rect::new(pos.x, pos.y, size.width, size.height);
            //let mut prev_pos = self.config.layout.find_anchor_pos(&monitor_rect, &Rect::new(0.0, 0.0, 0.0, 0.0));
            let mut prev_pos = monitor_rect.top_left().clone();
            prev_pos.x -= gap.x;
            prev_pos.y -= gap.y;

            // Windows which are marked for destroy should be overlapped so that destroying them will be less noticeable.
            for window in self.windows.iter().filter(|w| !w.marked_for_destroy) {
                window.set_position(prev_pos.x + gap.x, prev_pos.y + gap.y);

                let window_rect = window.get_rect();
                prev_pos = p.notification_hook.get_pos(&window_rect);
            }
        } else {
            // Panic because the config must have not been setup properly.
            panic!("The root LayoutBlock must be a NotificationBlock!");
        }

        // Outer state is now up to date with internal state.
        self.dirty = false;
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        self.windows.retain(|w| w.winit.id() != window_id);
        self.dirty = true;

        /*
        let index = self.windows.iter().position(|w| w.winit.id() == window_id);
        if let Some(idx) = index {
            //let win = self.windows.remove(idx);
            self.windows.remove(idx);
            // @IMPORTANT: Panics caused by not dropping both of these:
            // `Failed to lookup raw keysm: XError { ... }`.
            // `Failed to destroy input context: XError { ... }`.
            //
            // @TODO: figure out why this happens and maybe file a bug report?
            // Maybe it's because they use the window handle? semi-race condition?  maybe they drop
            // the drawable for us without winit realising?
            //drop(win.context);
            //drop(win.surface);
        }
        */
    }

    // This feels heavy.
    // Draw an individual window (mostly for expose events).
    pub fn draw_window(&mut self, window_id: WindowId) {
        for window in self.windows.iter_mut() {
            if window.winit.id() == window_id {
                window.draw();
                break;
            }
        }
    }

    // Draw all windows.
    pub fn draw_windows(&mut self) {
        for window in &mut self.windows {
            window.draw();
        }
    }
}

