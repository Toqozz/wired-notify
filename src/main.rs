#[macro_use]
extern crate bitflags;

mod bus;
mod cli;
mod config;
mod manager;
#[rustfmt::skip]
mod maths_utility;
mod rendering;

use std::{
    env,
    io::ErrorKind,
    io::Write,
    time::{Duration, Instant},
    fs::OpenOptions,
    fs::File,
};

use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn, platform::unix::EventLoopExtUnix};

use bus::dbus::{Message, Notification};
use cli::ShouldRun;
use config::Config;
use dbus::message::MessageType;
use manager::NotifyWindowManager;
use notify::DebouncedEvent;

fn print_notification_to_file(notification: &Notification, file: &mut File) {
    let res = file.write(
        format!(
            "id:{}|app_name:{}|summary:{}|body:{}|urgency:{:?}|percentage:{:?}|time:{}|timeout:{}\n",
            notification.id,
            notification.app_name,
            notification.summary,
            notification.body,
            notification.urgency,
            notification.percentage,
            notification.time.timestamp(),
            notification.timeout,
        ).as_bytes()
    );

    match res {
        Ok(_) => (),
        Err(e) => eprintln!("Error writing to print file: {}", e),
    }

}

fn main() {
    let args: Vec<String> = env::args().collect();
    match cli::process_cli(args) {
        Ok(should_run) => match should_run {
            ShouldRun::Yes => (),
            ShouldRun::No => return,
        },
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    let maybe_watcher = Config::init();
    let mut maybe_print_file =
        Config::get()
            .print_to_file
            .as_ref()
            .map_or(None, |f| {
                let maybe_file = OpenOptions::new().write(true).create(true).truncate(true).open(f);
                match maybe_file {
                    Ok(f) => Some(f),
                    Err(e) => {
                        eprintln!("Couldn't open print file: {}", e);
                        None
                    }
                }
            });
    dbg!(&maybe_print_file);

    let maybe_listener = cli::init_socket_listener()
        .map_or_else(
            |e| {
                eprintln!("{}", e);
                None
            },
            |v| Some(v)
        );

    // Allows us to receive messages from dbus.
    let receiver = bus::dbus::init_connection();
    let dbus_connection = bus::dbus::get_connection();

    let mut event_loop = EventLoop::new_x11().expect("Couldn't create an X11 event loop.");
    let mut manager = NotifyWindowManager::new(&event_loop);

    let mut poll_interval = Duration::from_millis(Config::get().poll_interval);
    let mut prev_instant = Instant::now();
    event_loop.run_return(|event, event_loop, control_flow| {
        match event {
            Event::NewEvents(StartCause::Init) => {
                *control_flow = ControlFlow::WaitUntil(Instant::now() + poll_interval)
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let now = Instant::now();

                // Time passed since last loop.
                let time_passed = now - prev_instant;
                prev_instant = now;
                manager.update(time_passed);

                // Read wired socket signals.
                // Since we're non-blocking, mostly this is just std::io::ErrorKind::WouldBlock.
                // For other errors, we should probably inform users to aide debugging.
                // I don't love the idea of spamming stderr here, however.
                if let Some(listener) = &maybe_listener {
                    match listener.accept() {
                        Ok((socket, _addr)) =>
                            match cli::handle_socket_message(&mut manager, event_loop, socket) {
                                Ok(_) => {},
                                Err(e) => {
                                    eprintln!("Error while handling socket message: {:?}", e);
                                }
                            },
                        Err(e) => {
                            if e.kind() != ErrorKind::WouldBlock {
                                eprintln!("{}", e);
                            }
                        }
                    }
                };

                // Check dbus signals.
                // If we don't do get incoming signals, notify sender will block when sending.
                let signal = dbus_connection.incoming(0).next();
                if let Some(message) = signal {
                    if message.msg_type() == MessageType::Signal
                        && &*message.interface().unwrap() == "org.freedesktop.DBus"
                        && &*message.member().unwrap() == "NameAcquired"
                        && &*message.get1::<&str>().unwrap() == "org.freedesktop.Notifications"
                    {
                        println!("DBus Init Success.");
                    }
                }

                // Receives `Notification`s from dbus.
                if let Ok(msg) = receiver.try_recv() {
                    match msg {
                        Message::Close(id) => {
                            if Config::get().closing_enabled {
                                manager.drop_notification(id);
                            }
                        }
                        Message::Notify(n) => {
                            if let Some(print_file) = &mut maybe_print_file {
                                print_notification_to_file(&n, print_file);
                            }

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
                            DebouncedEvent::Write(p)
                            | DebouncedEvent::Create(p)
                            | DebouncedEvent::Chmod(p) => {
                                if let Some(file_name) = p.file_name() {
                                    // Make sure the file that was changed is our file.
                                    if file_name == "wired.ron" && Config::try_reload(p) {
                                        // Success.
                                        poll_interval = Duration::from_millis(Config::get().poll_interval);
                                        manager.new_notification(
                                            Notification::from_self("Wired", "Config was reloaded.", 5000),
                                            event_loop,
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Restart timer for next loop.
                *control_flow = ControlFlow::WaitUntil(now + poll_interval);
            }

            Event::RedrawRequested(window_id) => {
                manager.request_redraw(window_id);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { window_id, event, .. } => manager.process_event(window_id, event),

            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => (), //_ => *control_flow = ControlFlow::Poll,
        }
    });
}
