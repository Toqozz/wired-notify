#[macro_use]
extern crate bitflags;

mod bus;
mod cli;
mod config;
mod icons;
mod manager;
#[rustfmt::skip]
mod maths_utility;
mod rendering;

use std::io::BufWriter;
use std::os::unix::net::UnixStream;
use std::thread::{self};
use std::{
    env,
    fs::File,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

use winit::event_loop::EventLoop;
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::run_on_demand::EventLoopExtRunOnDemand,
};

use bus::dbus::{Message, Notification};
use cli::ShouldRun;
use config::Config;
use home_dir::HomeDirExt;
use manager::NotifyWindowManager;

use crate::bus::dbus::Timeout;
use crate::cli::{handle_cli_command, CliCommand};
use crate::config::CONFIG;

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
    if let Some(filename) = CONFIG.load().print_to_file.as_ref() {
        let maybe_path = PathBuf::from(filename).expand_home();
        let expanded_filename = match maybe_path {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed tilde expansion: {}", e);
                return None;
            }
        };
        let maybe_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(expanded_filename);
        match maybe_file {
            Ok(f) => return Some(f),
            Err(e) => {
                eprintln!("Couldn't open print file: {}", e);
            }
        }
    }

    None
}

// User events in the event_loop.
#[derive(Debug)]
pub enum NotifyEvent {
    ConfigReload,
    DbusMessage(bus::dbus::Message),
    SocketCommand(CliCommand, UnixStream),
}

fn main() {
    // If any thread panics, we want to kill the process.
    // https://stackoverflow.com/questions/35988775/how-can-i-cause-a-panic-on-a-thread-to-immediately-end-the-main-thread
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let args: Vec<String> = env::args().collect();
    let config_path = match cli::process_cli(args) {
        Ok(should_run) => match should_run {
            ShouldRun::Yes(config_path) => config_path,
            ShouldRun::No => return,
        },
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    let mut event_loop: EventLoop<NotifyEvent> = EventLoopBuilder::with_user_event()
        .build()
        .expect("Couldn't create an X11 event loop.");

    let dbus_message_event = event_loop.create_proxy();
    let _dbus_thread_handle = bus::dbus::init_dbus_thread(dbus_message_event);

    let config_update_event = event_loop.create_proxy();
    if let Some(watcher) = Config::init(config_path) {
        thread::spawn(move || loop {
            if watcher.check_and_update_config() && CONFIG.load().notify_on_reload {
                config_update_event.send_event(NotifyEvent::ConfigReload).unwrap();
            }
        });
    };

    let socket_message_event = event_loop.create_proxy();
    match cli::CLIListener::init() {
        Ok(listener) => {
            thread::spawn(move || {
                listener.handle(socket_message_event);
            });
        }
        Err(e) => {
            eprintln!("Couldn't init CLIListener: {:?}", e);
        }
    }

    let mut manager = NotifyWindowManager::new(&event_loop);
    let mut prev_instant = Instant::now();
    event_loop
        .run_on_demand(|event, elwt| {
            macro_rules! update_manager {
                () => {
                    let now = Instant::now();

                    // Time passed since last loop.
                    let time_passed = now - prev_instant;
                    prev_instant = now;
                    manager.update(time_passed);
                };
            }
            match event {
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                    update_manager!();

                    // Wait until the next notification because there are no more notifications.
                    // A new notification will change this to polling.
                    if !manager.has_windows() {
                        elwt.set_control_flow(ControlFlow::Wait);
                    } else {
                        elwt.set_control_flow(ControlFlow::WaitUntil(
                            Instant::now() + Duration::from_millis(CONFIG.load().poll_interval),
                        ));
                    }
                }

                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    // Sometimes this causes double draws (we draw on spawn organically), but it's better
                    // to listen anyway.
                    manager.request_redraw(window_id);
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                Event::WindowEvent { window_id, event, .. } => manager.process_event(window_id, event),

                Event::UserEvent(uv) => {
                    let mut has_new_notification = false;
                    match uv {
                        NotifyEvent::ConfigReload => {
                            has_new_notification = true;
                            manager.file_handle = open_print_file();
                            manager.replace_or_spawn(
                                Notification::from_self(
                                    "Wired",
                                    "Config was reloaded.",
                                    Timeout::Milliseconds(5000),
                                ),
                                elwt,
                            );
                        }

                        NotifyEvent::DbusMessage(dbus_message) => {
                            has_new_notification = true;
                            match dbus_message {
                                Message::Close(id) => {
                                    if CONFIG.load().closing_enabled {
                                        manager.drop_notification(id);
                                    }
                                }
                                Message::Notify(n) => {
                                    if let Some(print_file) = &mut manager.file_handle {
                                        try_print_to_file(&n, print_file);
                                    }
                                    manager.replace_or_spawn(n, elwt);
                                }
                            }
                        }

                        NotifyEvent::SocketCommand(command, stream) => {
                            let writer = BufWriter::new(stream);
                            match handle_cli_command(&mut manager, elwt, command, writer) {
                                Ok(_) => {}
                                Err(e) => eprintln!("Error while handling received message: {:?}", e),
                            };
                        }
                    }

                    if has_new_notification {
                        elwt.set_control_flow(ControlFlow::WaitUntil(
                            Instant::now() + Duration::from_millis(CONFIG.load().poll_interval),
                        ));
                    }
                    // Update manager after getting a notification or a state change.
                    update_manager!();
                }
                _ => (),
            }
        })
        .expect("Event loop error");
}
