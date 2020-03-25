use winit::{
    event_loop::EventLoopWindowTarget,
    window::WindowId,
};

use crate::bus::dbus::DBusNotification;
use crate::config::Config;
use crate::rendering::window::NotifyWindow;
use crate::types::maths::Rect;
use crate::rendering::layout::{LayoutElement, LayoutBlock};
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

        let mut window = NotifyWindow::new(&self.config, el, notification);
        //let (rect, delta) = window.predict_size();
        //window.set_size(rect.width(),rect.height());
        //let pos = &self.config.layout.find_anchor_pos(&Rect::new(0., 0., 0., 0.), &rect);
        //dbg!(&pos);
        //window.set_position(pos.x, pos.y);
    //pub fn find_anchor_pos(&self, parent_rect: &Rect, self_rect: &Rect) -> Vec2 {
        //window.master_offset = delta;

        self.windows.push(window);

        // Outer state is now out of sync with internal state because we have an invisible notification.
        self.dirty = true;
    }

    pub fn update(&mut self, delta_time: Duration) {
        for window in &mut self.windows {
            self.dirty |= window.update(delta_time);
        }

        if self.dirty {
            self.update_positions();
            // Finally drop windows.
            self.windows.retain(|w| !w.marked_for_destroy);
        }
    }

    fn update_positions(&mut self) {
        if let LayoutElement::NotificationBlock(p) = &self.config.layout.params {
            let gap = &p.gap;
            let monitor = self.config.monitor.as_ref().expect("No monitor defined.");

            let (pos, size) = (monitor.position(), monitor.size());
            let monitor_rect = Rect::new(pos.x.into(), pos.y.into(), size.width.into(), size.height.into());

            let mut prev_pos = LayoutBlock::find_anchor_pos(
                &self.config.layout.hook,
                &self.config.layout.offset,
                &monitor_rect,
                &Rect::new(0.0, 0.0, 0.0, 0.0)
            );
            //let mut prev_pos = monitor_rect.top_left().clone();
            prev_pos.x -= gap.x;
            prev_pos.y -= gap.y;

            // Windows which are marked for destroy should be overlapped so that destroying them
            // will be less noticeable.
            for i in 0..self.windows.len() {
                let window = &self.windows[i];

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
        } else {
            // Panic because the config must have not been setup properly.
            panic!("The root LayoutElement must be a NotificationBlock!");
        }

        // Outer state is now up to date with internal state.
        self.dirty = false;
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        for window in &mut self.windows {
            if window.winit.id() == window_id {
                window.marked_for_destroy = true;
                self.dirty = true;

                break;
            }
        }
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

