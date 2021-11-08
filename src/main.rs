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
    fs::File,
    fs::OpenOptions,
    io::Write,
    time::{Duration, Instant},
};

use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    platform::unix::EventLoopExtUnix,
};

use bus::dbus::{Message, Notification};
use cli::ShouldRun;
use config::Config;
use dbus::message::MessageType;
use manager::NotifyWindowManager;

fn try_print_to_file(notification: &Notification, file: &mut File) {
    let json_string = match serde_json::to_string(&notification) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error serializing notification: {}", e);
            return;
        }
    };

    match writeln!(file, "{}", json_string) {
        Ok(_) => (),
        Err(e) => eprintln!("Error writing to print file: {}", e),
    }
}

fn open_print_file() -> Option<File> {
    if let Some(filename) = Config::get().print_to_file.as_ref() {
        let maybe_file = OpenOptions::new().write(true).create(true).truncate(true).open(filename);
        match maybe_file {
            Ok(f) => return Some(f),
            Err(e) => {
                eprintln!("Couldn't open print file: {}", e);
            }
        }
    }

    None
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
    let mut maybe_print_file = open_print_file();

    let maybe_listener = cli::CLIListener::init().map_or_else(
        |e| {
            eprintln!("Couldn't init CLIListener: {:?}", e);
            None
        },
        Some
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

                // TODO: be smarter about looping when no notifications are present.
                // TODO: clean this loop up

                // Time passed since last loop.
                let time_passed = now - prev_instant;
                prev_instant = now;
                manager.update(time_passed);

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

                // Read wired socket signals, for cli stuff.
                if let Some(listener) = &maybe_listener {
                    listener.process_messages(&mut manager, event_loop);
                };

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
                                try_print_to_file(&n, print_file);
                            }

                            manager.replace_or_spawn(n, event_loop);
                            /*
                            if Config::get().replacing_enabled && manager.notification_exists(&n) {
                                manager.replace_notification(n);
                            } else {
                                manager.new_notification(n, event_loop);
                            }
                            */
                        }
                    }
                }

                // Watch config file for changes.
                if let Some(cw) = &maybe_watcher {
                    // Config was changed, update some internal stuff.
                    if cw.check_and_update_config() {
                        poll_interval = Duration::from_millis(Config::get().poll_interval);
                        maybe_print_file = open_print_file();
                        manager.new_notification(
                            Notification::from_self("Wired", "Config was reloaded.", 5000),
                            event_loop,
                        );
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
