use winit::{
    event_loop::EventLoopWindowTarget,
    window::WindowId,
};

use crate::bus::dbus::Notification;
use crate::config::Config;
use crate::rendering::window::NotifyWindow;

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

    // TODO: Think about supporting horizontal notifications... do people even want that?
    pub fn update_positions(&mut self) {
        let (begin_posx, begin_posy) = (self.config.notification.x, self.config.notification.y);
        let gap = self.config.gap;

        let mut prev_y = begin_posy - gap;
        for window in self.windows.iter_mut() {
            window.set_position(begin_posx as f64, (prev_y + gap) as f64);

            prev_y = window.get_rect().bottom() as i32;
        }
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        let index = self.windows.iter().position(|w| w.winit.id() == window_id);
        if let Some(idx) = index {
            let win = self.windows.remove(idx);
            // @IMPORTANT: Panics caused by not dropping both of these:
            // `Failed to lookup raw keysm: XError { ... }`.
            // `Failed to destroy input context: XError { ... }`.
            //
            // @TODO: figure out why this happens and maybe file a bug report?
            // Maybe it's because they use the window handle? semi-race condition?  maybe they drop
            // the drawable for us without winit realising?
            drop(win.context);
            drop(win.surface);
        }
    }

    pub fn draw_windows(&mut self) {
        for window in self.windows.iter_mut() {
            if window.dirty {
                window.draw();
                window.dirty = false;
            }
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

