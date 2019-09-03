extern crate winit;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;
mod types;

use std::sync::mpsc;

//use winit::EventsLoop;
use winit::{
    event::{ Event, WindowEvent },
    event_loop::{ ControlFlow, EventLoop, EventLoopWindowTarget },
    platform::desktop::EventLoopExtDesktop,
};

use notification::management::NotifyWindowManager;
//use bus::dbus::Notification;

//fn spawn_window(notification: Notification, manager: &mut NotifyWindowManager, el: &EventLoopWindowTarget<()>) {
    //manager.new_notification(notification, el);
//}

fn main() {
    // Hack to avoid winit dpi scaling -- we just want pixels.
    std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

    let mut event_loop = EventLoop::new();    // TODO: maybe use `EventsLoop::new_x11()` ?

    let mut config: config::Config = ron::de::from_str(include_str!("config.ron"))
        .expect("Failed to load config.\n");
    //config.notification.layout = config::construct_layouts(&config.notification.root);

    let mut manager = NotifyWindowManager::new(&config);

    // Allows us to receive messages from dbus.
    let (sender, receiver) = mpsc::channel();
    let connection = bus::dbus::dbus_loop(sender);

    event_loop.run_return(move |event, event_loop, control_flow| {
        match event {
            Event::EventsCleared => {
                // Application update code.
                manager.draw_windows();

                // Check dbus signals.
                let signal = connection.incoming(0).next();
                if let Some(s) = signal {
                    dbg!(s);
                }

                if let Ok(x) = receiver.try_recv() {
                    //spawn_window(x, &mut manager, &event_loop);
                    manager.new_notification(x, event_loop);
                    // Initial draw, otherwise we won't redraw until the event queue clears again.
                    // @NOTE: is this an issue for framerate draws? -- investigate winit timer.
                    manager.draw_windows();
                }

                // @TODO: figure out why uncommenting this causes draw_windows to not draw?
                //std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
            },
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                println!("Close requested; stopping.");
                *control_flow = ControlFlow::Exit
            },
            Event::WindowEvent { window_id, event: WindowEvent::MouseInput { .. } } => {
                manager.drop_window(window_id);
            },

            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
