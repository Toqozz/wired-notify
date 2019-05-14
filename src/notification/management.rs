use sdl2::EventPump;

use crate::rendering::window::{ SDL2Window, };
use crate::rendering::sdl::{ SDL2State, };

use crate::bus::dbus::Notification;
use crate::config::Config;

#[derive(Debug, Clone)]
struct Vec2 {
    x: f64,
    y: f64,
}

pub struct NotifyWindow {
    pub window: SDL2Window,
    pub notification: Notification,

    // Positioning.
    position: Vec2,     // x, y
    size: Vec2,         // width, height

    // Timeout.
    fuse: f32,
}

impl NotifyWindow {
    pub fn new(window: SDL2Window, notification: Notification) -> Self {
        Self {
            window,
            notification,
            position: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 500.0, y: 60.0 },
            fuse: 0.0,
        }
    }
}

pub struct NotifyWindowManager<'a> {
    pub sdl: SDL2State,
    pub notify_windows: Vec<NotifyWindow>,

    pub config: &'a Config,
}


impl<'a> NotifyWindowManager<'a> {
    pub fn new(config: &'a Config) -> (Self, EventPump) {
        let (sdl, ev) = SDL2State::new()
            .expect("Failed to create SDL2State.");

        let notify_windows = Vec::new();

        (Self { sdl, notify_windows, config }, ev)
    }

    pub fn drop_window(&mut self, window_id: u32) {
        let position = self.notify_windows.iter().position(|n| n.window.canvas.window().id() == window_id);
        if let Some(pos) = position {
            self.notify_windows.remove(pos);
        }
    }

    pub fn draw_windows(&mut self) {
        for notify_window in self.notify_windows.iter_mut() {
            notify_window.window.draw();
        }
    }

    pub fn new_notification(&mut self, notification: Notification) {
        let window = SDL2Window::new(&self.sdl, &self.config)
            .expect("Could not create SDL2Window.");
        let notify_window = NotifyWindow::new(window, notification);

        self.notify_windows.push(notify_window);
    }
}

