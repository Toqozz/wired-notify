use winit::EventsLoop;
use winit::WindowId;
use winit::dpi::LogicalPosition;

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
    //position: Vec2,     // x, y
    //size: Vec2,         // width, height

    // Timeout.
    //fuse: f32,
}

impl NotifyWindow {
    pub fn new(window: SDL2Window, notification: Notification) -> Self {
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
    pub notify_windows: Vec<NotifyWindow>,

    pub config: &'a Config,
    //pub events_loop: &'a EventsLoop,
}


impl<'a> NotifyWindowManager<'a> {
    pub fn new(config: &'a Config, sdl: &'a SDL2State) -> Self {
        //let sdl = SDL2State::new()
            //.expect("Failed to create SDL2State.");

        let notify_windows = Vec::new();

        Self { sdl, notify_windows, config }
    }

    pub fn update_positions(&mut self) {
        let begin_posx = self.config.notification.x as f64;
        let begin_posy = self.config.notification.y as f64;
        let height = self.config.notification.height as f64;
        let gap = self.config.gap as f64;
        for (i, notify_window) in self.notify_windows.iter_mut().enumerate() {
            let num = i as f64;
            let pos = LogicalPosition { x: begin_posx, y: begin_posy + num * (height + gap) };
            notify_window.window.set_position(pos);
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
            notify_window.window.draw_text(self.sdl, self.config, notify_window.notification.summary.as_str());
        }
    }

    pub fn new_notification(&mut self, notification: Notification, el: &EventsLoop) {
        let window = SDL2Window::new(&self.sdl, &self.config, el)
            .expect("Could not create SDL2Window.");
        let notify_window = NotifyWindow::new(window, notification);

        self.notify_windows.push(notify_window);
        // NOTE: I think that this is expensive when there's a lot of notifications.
        self.update_positions();
    }
}

