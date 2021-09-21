use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::panic::panic_any;
use std::path::Path;

use getopts::Options;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowId;

use crate::NotifyWindowManager;

pub const SOCKET_PATH: &str = "/tmp/wired.sock";

#[derive(Debug)]
pub enum SocketError {
    Parse(&'static str),
    NotificationNotFound,
    InvalidCommand,
}

pub enum ShouldRun {
    Yes,
    No,
}

pub fn init_socket_listener() -> Result<UnixListener, String> {
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
            return Err(format!("Couldn't bind socket {}\n{:?}", SOCKET_PATH, e));
        }
    };
    listener.set_nonblocking(true).unwrap();
    Ok(listener)
}

// Socket stuff:
fn get_window_id(arg: &str, manager: &NotifyWindowManager) -> Result<WindowId, SocketError> {
    let idx = if arg == "latest" {
        let num_windows = manager.monitor_windows.values().flatten().count();
        if num_windows > 0 {
            num_windows - 1
        } else {
            return Err(SocketError::NotificationNotFound);
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

pub fn handle_socket_message(
    manager: &mut NotifyWindowManager,
    el: &EventLoopWindowTarget<()>,
    stream: UnixStream,
) -> Result<(), SocketError> {
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
                    let (notif_id, action_id) = args
                        .split_once(",")
                        .ok_or(SocketError::Parse("Malformed action request."))?;

                    let id = get_window_id(notif_id, manager)?;
                    let action = action_id
                        .parse::<usize>()
                        .map_err(|_| SocketError::Parse("Value is not of type usize."))?;
                    manager.trigger_action_idx(id, action);
                }
                "show" => {
                    let num = args
                        .parse::<usize>()
                        .map_err(|_| SocketError::Parse("Value is not of type usize."))?;
                    for _ in 0..num {
                        if let Some(n) = manager.history.pop_back() {
                            manager.new_notification(n, el);
                        }
                    }
                }
                _ => return Err(SocketError::InvalidCommand),
            }
        } else {
            return Err(SocketError::Parse("Malformed command."));
        }
    }

    Ok(())
}

// CLI stuff:
fn print_usage(opts: Options) {
    print!(
        "{}",
        opts.usage("Usage:\twired [options]\n\tIDX refers to the Nth most recent notification.")
    );
}

fn validate_identifier(input: &str, allow_all: bool) -> Result<(), &'static str> {
    if input == "latest" || (input == "all" && allow_all) {
        return Ok(());
    }

    // We don't actually care about the value here -- this is just client side validation.
    match input.parse::<u32>() {
        Ok(_) => Ok(()),
        Err(_) => Err("Notification identifier must be either [latest], or a valid \
                                notification IDX (unsigned integer)."),
    }
}

fn validate_action(input: &str) -> Result<(), &'static str> {
    if ["default", "1", "2", "3"].contains(&input) {
        Ok(())
    } else {
        Err("Notification action must be one of [default|1|2|3].")
    }
}

pub fn process_cli(args: Vec<String>) -> Result<ShouldRun, String> {
    if args.len() == 1 {
        // No options, assume --run.
        return Ok(ShouldRun::Yes);
    }

    // Initialization
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("d", "drop", "drop/close a notification", "[latest|all|IDX]");
    opts.optopt(
        "a",
        "action",
        "execute a notification's action",
        "[latest|IDX]:[default|1|2|3]",
    );
    opts.optopt("s", "show", "show the last N notifications", "N");
    opts.optflag("r", "run", "run the wired daemon");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic_any(e.to_string()),
    };

    // Matching
    if matches.opt_present("h") {
        print_usage(opts);
        return Ok(ShouldRun::No);
    }

    if matches.opt_present("r") {
        return Ok(ShouldRun::Yes);
    }

    // All these options use a socket.
    if matches.opt_present("d") || matches.opt_present("a") || matches.opt_present("s") {
        let mut sock = match UnixStream::connect(SOCKET_PATH) {
            Ok(s) => s,
            Err(e) => {
                return Err(format!(
                    "Tried to send a command to the wired socket but couldn't connect; \
                         is the wired daemon running?\n{}",
                    e
                ))
            }
        };

        if let Some(to_close) = matches.opt_str("d") {
            validate_identifier(to_close.as_str(), true)?;
            sock.write(format!("close:{}", to_close).as_bytes())
                .map_err(|e| e.to_string())?;
        }

        if let Some(to_action) = matches.opt_str("a") {
            let (notification, action) = match to_action.split_once(":") {
                Some(na) => na,
                None => {
                    return Err("Missing ':' in action argument.\n\
                            Notification and action arguments must be in the format of \
                            [latest|N]:[default|1|2|3]"
                        .to_owned());
                }
            };

            validate_identifier(notification, false)?;
            validate_action(action)?;
            sock.write(format!("action:{},{}", notification, action).as_bytes())
                .map_err(|e| e.to_string())?;
        }

        if let Some(to_show) = matches.opt_str("s") {
            validate_identifier(to_show.as_str(), false)?;
            sock.write(format!("show:{}", to_show).as_bytes())
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(ShouldRun::No)
}
