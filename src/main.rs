extern crate winit;
extern crate dirs;
extern crate wiry_derive;

mod rendering;
mod notification;
mod bus;
mod config;
mod maths;

use std::time::{Instant, Duration};

use winit::{
    event::{ StartCause, Event, WindowEvent, ElementState, MouseButton },
    event_loop::{ ControlFlow, EventLoop },
    platform::desktop::EventLoopExtDesktop,
    platform::unix::EventLoopExtUnix,
};
use notify::DebouncedEvent;
use dbus::message::MessageType;

use config::Config;
use notification::management::NotifyWindowManager;

fn main() {
    // Hack to avoid winit dpi scaling -- we just want pixels.
    std::env::set_var("WINIT_X11_SCALE_FACTOR", "1.0");

    let mut event_loop = EventLoop::new_x11().expect("Couldn't create an X11 event loop.");

    //let config = Config::load().unwrap();
    //let maybe_watcher = Config::watch();
    let maybe_watcher = Config::init();

    let mut manager = NotifyWindowManager::new();

    // Allows us to receive messages from dbus.
    let (connection, receiver) = bus::dbus::get_connection();

    let timer_length = Duration::from_millis(Config::get().poll_interval);
    let mut prev_instant = Instant::now();
    event_loop.run_return(move |event, event_loop, control_flow| {
        match event {
            // @NOTE: maybe we should separate receiving dbus signals and drawing windows.
            Event::NewEvents(StartCause::Init) => *control_flow = ControlFlow::WaitUntil(Instant::now() + timer_length),
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let now = Instant::now();

                // Time passed since last loop.
                let time_passed = now - prev_instant;
                prev_instant = now;
                manager.update(time_passed);

                // Check dbus signals.
                // If we don't do get incoming signals, notify sender will block when sending.
                let signal = connection.incoming(0).next();
                if let Some(message) = signal {
                    if message.msg_type() == MessageType::Signal &&
                       &*message.interface().unwrap() == "org.freedesktop.DBus" &&
                       &*message.member().unwrap() == "NameAcquired" &&
                       &*message.get1::<&str>().unwrap() == "org.freedesktop.Notifications" {
                        println!("Name acquired.");
                    }
                }

                if let Ok(x) = receiver.try_recv() {
                    //spawn_window(x, &mut manager, &event_loop);
                    manager.new_notification(x, event_loop);
                }

                // If the watcher exists (.config/wiry exists), then we should process watcher events.
                if let Some(cw) = &maybe_watcher {
                    if let Ok(ev) = cw.receiver.try_recv() {
                        // @TODO: print a notification when config reloaded?
                        match ev {
                            DebouncedEvent::Write(_) |
                            DebouncedEvent::Create(_) |
                            DebouncedEvent::Chmod(_) => {
                                Config::try_reload();
                            },
                            _ => {},
                        }
                    }
                }

                // Restart timer for next loop.
                *control_flow = ControlFlow::WaitUntil(now + timer_length);
            },

            // Window becomes visible and then position is set.  Need fix.
            Event::RedrawRequested(window_id) => manager.draw_window(window_id),
            Event::WindowEvent {
                window_id,
                event: WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. },
                ..
            } => {
                manager.drop_window(window_id);
            },
                    //state: ElementState::Pressed,
                    //button: MouseButton::Left,
                    //.. }
            //} => {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,

            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => ()
            //_ => *control_flow = ControlFlow::Poll,
        }
    });
}
