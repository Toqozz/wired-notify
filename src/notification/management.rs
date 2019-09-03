use winit::{
    event_loop::{ EventLoop, EventLoopWindowTarget },
    window::WindowId,
};

use crate::bus::dbus::Notification;
use crate::config::Config;
use crate::rendering::text::TextDrawable;
use crate::types::maths::Point;
use crate::rendering::window::NotifyWindow;

pub struct NotifyWindowManager<'config> {
    //pub notify_windows: Vec<NotifyWindow<'config>>,
    pub windows: Vec<NotifyWindow<'config>>,

    pub config: &'config Config,
    //pub events_loop: &'a EventsLoop,
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
        //let begin_posy = self.config.notification.y;
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
        let mut window = NotifyWindow::new(&self.config, el, notification);

        let ctx = &window.context;

        /*
        let summary_drawable = TextDrawable::new(
            ctx,
            window.notification.summary.clone(),
            self.config.notification.summary.padding.clone(),
            Point {
                x: self.config.notification.summary.offset.x,
                y: self.config.notification.summary.offset.y,
            },
        );

        let mut body_drawable = TextDrawable::new(
            ctx,
            window.notification.body.clone(),
            self.config.notification.body.padding.clone(),
            Point {
                x: self.config.notification.body.offset.x,
                y: self.config.notification.body.offset.y,
            },
        );

        body_drawable.set_anchor(
            &summary_drawable.get_anchor(&self.config.notification.body.anchor_position)
        );
        */

        /*
        // Ugly but working.
        // Consider moving this calculation into a function.
        let r1 = summary_drawable.get_rect();
        let r2 = body_drawable.get_rect();
        let mut rect = r1.union(r2);
        rect.set_x(rect.x() - self.config.notification.border_width);
        rect.set_y(rect.y() - self.config.notification.border_width);
        rect.set_width(rect.width() + self.config.notification.border_width * 2f64);
        rect.set_height(rect.height() + self.config.notification.border_width * 2f64);
        */

        //window.set_size(rect.width(), rect.height());
        let rect = window.predict_size();
        window.set_size(rect.width(),rect.height());

        //window.drawables.push(summary_drawable);
        //window.drawables.push(body_drawable);

        self.windows.push(window);
        // NOTE: I think that this is expensive when there's a lot of notifications.
        self.update_positions();

        //notify_window.window.draw();
        //notify_window.window.draw_text(notify_window.notification.summary.as_str(), notify_window.notification.body.as_str());

    }
}

