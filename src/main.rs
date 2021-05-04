#[macro_use]
extern crate bitflags;

mod rendering;
mod management;
mod bus;
mod config;
mod maths_utility;

use std::time::{Instant, Duration};

use winit::{
    event::{StartCause, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
    platform::unix::EventLoopExtUnix,
};
use notify::DebouncedEvent;
use dbus::message::MessageType;

use config::Config;
use management::NotifyWindowManager;
use wired_derive;

fn main() {
    // Hack to avoid winit dpi scaling -- we just want pixels.
    std::env::set_var("WINIT_X11_SCALE_FACTOR", "1.0");

    let maybe_watcher = Config::init();

    let mut event_loop = EventLoop::new_x11().expect("Couldn't create an X11 event loop.");
    let mut manager = NotifyWindowManager::new(&event_loop);

    // Allows us to receive messages from dbus.
    let (connection, receiver) = bus::dbus::get_connection();

    let mut poll_interval = Duration::from_millis(Config::get().poll_interval);
    let mut prev_instant = Instant::now();
    event_loop.run_return(move |event, event_loop, control_flow| {
        match event {
            // @NOTE: maybe we should separate receiving dbus signals and drawing windows.
            Event::NewEvents(StartCause::Init) => *control_flow = ControlFlow::WaitUntil(Instant::now() + poll_interval),
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

                let x = bus::dbus_codegen::OrgFreedesktopNotificationsActionInvoked { action_key: "default".to_owned(), id: 0 };
                use dbus::message::SignalArgs;
                let path = dbus::strings::Path::new("/org/freedesktop/Notifications").unwrap();
                let res = connection.send(x.to_emit_message(&path));
                dbg!(res);

                if let Ok(x) = receiver.try_recv() {
                    //spawn_window(x, &mut manager, &event_loop);
                    manager.new_notification(x, event_loop);
                }

                // If the watcher exists, then we should process watcher events.
                if let Some(cw) = &maybe_watcher {
                    if let Ok(ev) = cw.receiver.try_recv() {
                        // @TODO: print a notification when config reloaded?
                        match ev {
                            DebouncedEvent::Write(p) |
                            DebouncedEvent::Create(p) |
                            DebouncedEvent::Chmod(p) => {
                                if let Some(file_name) = p.file_name() {
                                    // Make sure the file that was changed is our file.
                                    if file_name == "wired.ron" {
                                        Config::try_reload(p);
                                        poll_interval = Duration::from_millis(Config::get().poll_interval);
                                    }
                                }
                            },
                            _ => {},
                        }
                    }
                }

                // Restart timer for next loop.
                *control_flow = ControlFlow::WaitUntil(now + poll_interval);
            },

            // Window becomes visible and then position is set.  Need fix.
            Event::RedrawRequested(window_id) => manager.draw_window(window_id),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,

            // TODO: fix this givinng whole window event.
            Event::WindowEvent { window_id, event, .. } => manager.process_event(window_id, event),


            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => ()
            //_ => *control_flow = ControlFlow::Poll,
        }
    });
}
