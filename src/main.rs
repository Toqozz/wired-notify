#[macro_use]
extern crate bitflags;

mod rendering;
mod management;
mod bus;
mod config;
mod maths_utility;

use std::env;
use std::time::{Instant, Duration};
use std::os::unix::net::{ UnixStream, UnixListener };
use std::io::{BufRead, BufReader};

use winit::{
    event::{StartCause, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
    platform::unix::EventLoopExtUnix,
};
use notify::DebouncedEvent;
use dbus::message::MessageType;
use bus::dbus::{ Message, Notification };

use config::Config;
use management::NotifyWindowManager;
use wired_derive;

/*
fn run_daemon() {

}
*/

fn main() {
    let maybe_watcher = Config::init();

    // Socket, for listening to CLI calls to ourselves.
    /*
    std::fs::remove_file("/tmp/wired.sock").unwrap();
    let socket_listener = match UnixListener::bind("/tmp/wired.sock") {
        Ok(sock) => sock,
        Err(e) => {
            eprintln!("Couldn't bind socket /tmp/wired.sock, is another wired instance running?\n{:?}", e);
            return;
        }
    };
    */

    // Allows us to receive messages from dbus.
    let receiver = bus::dbus::init_connection();
    let dbus_connection = bus::dbus::get_connection();

    let mut event_loop = EventLoop::new_x11().expect("Couldn't create an X11 event loop.");
    let mut manager = NotifyWindowManager::new(&event_loop);

    let mut poll_interval = Duration::from_millis(Config::get().poll_interval);
    let mut prev_instant = Instant::now();
    event_loop.run_return(|event, event_loop, control_flow| {
        match event {
            Event::NewEvents(StartCause::Init) => *control_flow = ControlFlow::WaitUntil(Instant::now() + poll_interval),
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let now = Instant::now();

                // Time passed since last loop.
                let time_passed = now - prev_instant;
                prev_instant = now;
                manager.update(time_passed);

                // Read wired socket signals.
                /*
                for stream in socket_listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            let stream = BufReader::new(stream);
                            for line in stream.lines() {
                                println!("{}", line.unwrap());
                            }
                        },

                        Err(e) => {
                            dbg!("Error reading stream.");
                            break;
                        }
                    }
                }
                */

                // Check dbus signals.
                // If we don't do get incoming signals, notify sender will block when sending.
                let signal = dbus_connection.incoming(0).next();
                if let Some(message) = signal {
                    if message.msg_type() == MessageType::Signal &&
                       &*message.interface().unwrap() == "org.freedesktop.DBus" &&
                       &*message.member().unwrap() == "NameAcquired" &&
                       &*message.get1::<&str>().unwrap() == "org.freedesktop.Notifications" {
                        println!("DBus Init Success.");
                    }
                }

                // Receives `Notification`s from dbus.
                if let Ok(msg) = receiver.try_recv() {
                    match msg {
                        Message::Close(id) => {
                            if Config::get().closing_enabled { manager.drop_notification(id); }
                        },
                        Message::Notify(n) => {
                            if Config::get().replacing_enabled && manager.notification_exists(n.id) {
                                manager.replace_notification(n);
                            } else {
                                manager.new_notification(n, event_loop);
                            }
                        }
                    }
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
                                        if Config::try_reload(p) {
                                            // Success.
                                            poll_interval = Duration::from_millis(Config::get().poll_interval);
                                            manager.new_notification(
                                                Notification::from_self("Wired", "Config was reloaded.", 5000),
                                                event_loop,
                                            );
                                        }
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
            Event::RedrawRequested(window_id) => manager.request_redraw(window_id),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { window_id, event, .. } => manager.process_event(window_id, event),

            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => ()
            //_ => *control_flow = ControlFlow::Poll,
        }
    });
}
