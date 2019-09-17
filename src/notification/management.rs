use winit::{
    event_loop::EventLoopWindowTarget,
    window::WindowId,
};

use crate::bus::dbus::Notification;
use crate::config::Config;
use crate::rendering::window::NotifyWindow;
use crate::types::maths::{Vec2, Rect};
use crate::config::LayoutBlock::{self, NotificationBlock};
use crate::config::AnchorPosition;
use std::time::Duration;

pub struct NotifyWindowManager<'config> {
    pub windows: Vec<NotifyWindow<'config>>,

    pub config: &'config Config,
}

impl<'config> NotifyWindowManager<'config> {
    pub fn new(config: &'config Config) -> Self {
        //let notify_windows = Vec::new();
        let windows = vec![];

        Self {
            windows,
            config,
        }
    }

    pub fn update_timers(&mut self, time_passed: Duration) {
        println!("timers");
        let mut i = 0;
        while i < self.windows.len() {
            self.windows[i].notification.expire_timeout -= time_passed.as_millis() as i32;
            dbg!(self.windows[i].notification.expire_timeout);
            if self.windows[i].notification.expire_timeout > 0 {
                self.windows.remove(i);
            }

            i += 1;
        }
    }

    pub fn request_redraw(&self) {
        for window in &self.windows {
            window.winit.request_redraw();
        }
    }

    // TODO: Think about supporting horizontal notifications... do people even want that?
    pub fn update_positions(&mut self) {
        if let NotificationBlock(parameters) = &self.config.layout {
            let gap = &parameters.gap;
            let monitor = self.config.monitor.as_ref().expect("No monitor defined.");

            let (pos, size) = (monitor.position(), monitor.size());
            let monitor_rect = Rect::new(pos.x, pos.y, size.width, size.height);
            let mut prev_pos = match &parameters.monitor_hook {
                AnchorPosition::TL => monitor_rect.top_left(),
                AnchorPosition::TR => monitor_rect.top_right(),
                AnchorPosition::BL => monitor_rect.bottom_left(),
                AnchorPosition::BR => monitor_rect.bottom_right(),
            };
            prev_pos.x -= gap.x;
            prev_pos.y -= gap.y;

            for window in self.windows.iter() {
                window.set_position(prev_pos.x + gap.x, prev_pos.y + gap.y);

                let window_rect = window.get_rect();
                prev_pos = match &parameters.notification_hook {
                    AnchorPosition::TL => window_rect.top_left(),
                    AnchorPosition::TR => window_rect.top_right(),
                    AnchorPosition::BL => window_rect.bottom_left(),
                    AnchorPosition::BR => window_rect.bottom_right(),
                };
            }
        } else {
            // Panic because the config must have not been setup properly.
            panic!();
        }
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        self.windows.retain(|w| w.winit.id() != window_id);
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

    pub fn draw_windows(&mut self) {
        for window in self.windows.iter_mut() {
            window.draw();
            //if window.dirty {
            //}
        }
    }

    pub fn new_notification(&mut self, notification: Notification, el: &EventLoopWindowTarget<()>) {
        let window = NotifyWindow::new(&self.config, el, notification);
        let rect = window.predict_size();
        window.set_size(rect.width(),rect.height());

        self.windows.push(window);

        // IMPORTANT: Is this expensive when there is a lot of notifications?
        //  What about when we have to switch a bunch of notifications?
        self.update_positions();
    }
}

