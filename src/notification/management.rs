use winit::{
    EventsLoop,
    WindowId,
};

use crate::rendering::{
    window::SDL2Window,
    sdl::SDL2State,
};
use crate::bus::dbus::Notification;
use crate::config::Config;

#[derive(Debug, Clone)]
struct Vec2 {
    x: f64,
    y: f64,
}

pub struct NotifyWindow<'a> {
    pub window: SDL2Window<'a>,
    pub notification: Notification,

    // Positioning.
    //position: Vec2,     // x, y
    //size: Vec2,         // width, height

    // Timeout.
    //fuse: f32,
}

impl<'a> NotifyWindow<'a> {
    pub fn new(window: SDL2Window<'a>, notification: Notification) -> Self {
        Self {
            window,
            notification,
            //position: Vec2 { x: 0.0, y: 0.0 },
            //size: Vec2 { x: 500.0, y: 60.0 },
            //fuse: 0.0,
        }
    }
}

pub struct NotifyWindowManager<'a> {
    pub sdl: &'a SDL2State,
    pub notify_windows: Vec<NotifyWindow<'a>>,

    pub config: &'a Config,
    //pub events_loop: &'a EventsLoop,
}


impl<'a> NotifyWindowManager<'a> {
    pub fn new(config: &'a Config, sdl: &'a SDL2State) -> Self {
        let notify_windows = Vec::new();

        Self {
            sdl,
            notify_windows,
            config,
        }
    }

    // TODO: support vertical notifications.
    // Think about supporting horizontal notifications... do people even want that?
    pub fn update_positions(&mut self) {
        let begin_posx = self.config.notification.x;
        let begin_posy = self.config.notification.y;
        let gap = self.config.gap;

        let mut prev_y = begin_posy - gap;
        for notify_window in self.notify_windows.iter_mut() {
            let posx = sdl2::video::WindowPos::Positioned(begin_posx);
            let posy = sdl2::video::WindowPos::Positioned(prev_y + gap);
            notify_window.window.set_position(posx, posy);

            prev_y = notify_window.window.get_rect().bottom();
        }
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        let position = self.notify_windows.iter().position(|n| n.window.winit_window.id() == window_id);
        if let Some(pos) = position {
            self.notify_windows.remove(pos);
        }
    }

    pub fn draw_windows(&mut self) {
        for notify_window in self.notify_windows.iter_mut() {
            notify_window.window.draw();
            notify_window.window.draw_text(self.sdl, notify_window.notification.summary.as_str(), notify_window.notification.body.as_str());
        }
    }

    pub fn new_notification(&mut self, notification: Notification, el: &EventsLoop) {
        let window = SDL2Window::new(&self.sdl, &self.config, el)
            .expect("Could not create SDL2Window.");
        let mut notify_window = NotifyWindow::new(window, notification);

        notify_window.window.draw();
        notify_window.window.draw_text(self.sdl, notify_window.notification.summary.as_str(), notify_window.notification.body.as_str());

        self.notify_windows.push(notify_window);
        // NOTE: I think that this is expensive when there's a lot of notifications.
        self.update_positions();
    }
}

