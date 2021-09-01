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
    io::{BufRead, BufReader, ErrorKind},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    time::{Duration, Instant},
};

use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    platform::run_return::EventLoopExtRunReturn, platform::unix::EventLoopExtUnix, window::WindowId};

use bus::dbus::{Message, Notification};
use cli::ShouldRun;
use config::Config;
use dbus::message::MessageType;
use manager::NotifyWindowManager;
use notify::DebouncedEvent;

const SOCKET_PATH: &str = "/tmp/wired.sock";

#[derive(Debug)]
enum SocketError {
    Parse(&'static str),
    NotificationNotFound,
    InvalidCommand,
}

fn get_window_id(arg: &str, manager: &NotifyWindowManager) -> Result<WindowId, SocketError> {
    let idx = if arg == "latest" {
        let num_windows = manager.monitor_windows.values().flatten().count();
        if num_windows > 0 {
            num_windows - 1
        } else {
            return Err(SocketError::NotificationNotFound)
        }
    } else if let Ok(idx) = arg.parse::<usize>() {
        idx
    } else {
        return Err(SocketError::Parse("Value is not of type usize."));
    };

    if let Some(window) = manager.find_window_ordered(idx) {
        Ok(window)
    } else {
        Err(SocketError::NotificationNotFound)
    }
}

fn handle_socket_message(manager: &mut NotifyWindowManager, el: &EventLoopWindowTarget<()>, stream: UnixStream) -> Result<(), SocketError> {
    let stream = BufReader::new(stream);
    for line in stream.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        println!("Recived socket message: {}", line);
        if let Some((command, args)) = line.split_once(":") {
            match command {
                "close" => {
                    if args == "all" {
                        manager.drop_windows();
                    } else {
                        let id = get_window_id(args, manager)?;
                        manager.drop_window_id(id);
                    }
                }
                "action" => {
                    let (notif_id, action_id) =
                        args.split_once(",")
                            .ok_or(SocketError::Parse("Malformed action request."))?;

                    let id = get_window_id(notif_id, manager)?;
                    let action = action_id.parse::<usize>()
                        .map_err(|_| SocketError::Parse("Value is not of type usize."))?;
                    manager.trigger_action_idx(id, action);
                },
                "show" => {
                    let num = args.parse::<usize>()
                        .map_err(|_| SocketError::Parse("Value is not of type usize."))?;
                    for _ in 0..num {
                        if let Some(n) = manager.history.pop_back() {
                            manager.new_notification(n, el);
                        }
                    }
                }
                _ => {
                    return Err(SocketError::InvalidCommand)
                },
            }
        } else {
            return Err(SocketError::Parse("Malformed command."));
        }
    }

    Ok(())
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

    // Socket, for listening to CLI calls to ourselves.
    // We leave the socket up in pretty much all cases when closing, and just unbind it always.
    // This could cause confusing behavior for users where they have 2 wired instances running, and
    // neither will work properly (one will have the notification bus name, and one will have the
    // socket).
    // "Fixing" this would require making sure the socket is unbound in almost all cases -- likely
    // having to import a few crates like Ctrl-C and others -- yuck.
    // Let's just leave it as it is and try to communicate to users that it's not an issue.
    // https://stackoverflow.com/questions/40218416/how-do-i-close-a-unix-socket-in-rust
    let socket_path = Path::new(SOCKET_PATH);
    if socket_path.exists() {
        println!(
            "A wired socket exists; taking ownership.  Existing wired processes will not receive CLI calls."
        );
        std::fs::remove_file(SOCKET_PATH).unwrap();
    }
    let listener = match UnixListener::bind(socket_path) {
        Ok(sock) => sock,
        Err(e) => {
            eprintln!("Couldn't bind socket {}\n{:?}", SOCKET_PATH, e);
            return;
        }
    };
    listener.set_nonblocking(true).unwrap();

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
                match listener.accept() {
                    Ok((socket, _addr)) =>
                        match handle_socket_message(&mut manager, event_loop, socket) {
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
