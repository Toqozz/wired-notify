extern crate winit;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;

use std::sync::mpsc;

use winit::EventsLoop;

use notification::management::NotifyWindowManager;
use bus::dbus::Notification;

fn spawn_window(notification: Notification, manager: &mut NotifyWindowManager, el: &EventsLoop) {
    manager.new_notification(notification, el);
}

fn main() -> Result<(), String> {
    let mut events_loop = EventsLoop::new();    // TODO: maybe use `EventsLoop::new_x11()` ?

    let config: config::Config = toml::from_str(include_str!("config.toml"))
        .expect("Failed to load config.\n");

    let mut manager = NotifyWindowManager::new(&config);

    // Allows us to receive messages from dbus.
    let (sender, receiver) = mpsc::channel();
    let connection = bus::dbus::dbus_loop(sender);


    let mut running = true;
    while running {
        events_loop.poll_events(|event| {
            match event {
                winit::Event::WindowEvent {
                    window_id,
                    event: winit::WindowEvent::MouseInput { .. },   // NOTE: Can use modifiers here, like ctrl, shift, alt.
                } => manager.drop_window(window_id),
                winit::Event::WindowEvent {
                    event: winit::WindowEvent::CloseRequested,
                    ..
                } => running = false,
                _ => {}
            }
        });

        manager.draw_windows();

        // Check dbus signals.
        let signal = connection.incoming(0).next();
        if let Some(s) = signal {
            dbg!(s);
        }

        if let Ok(x) = receiver.try_recv() {
            spawn_window(x, &mut manager, &events_loop);
        }

        // Roughly 60fps.
        std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
    }

    Ok(())
}
