use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::panic::panic_any;
use std::path::Path;

use getopts::Options;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowId;

use crate::NotifyWindowManager;

pub const SOCKET_PATH: &str = "/tmp/wired.sock";

#[derive(Debug)]
pub enum CLIError {
    Parse(&'static str),
    NotificationNotFound,
    InvalidCommand,
    Socket(io::Error),
}

pub enum ShouldRun {
    Yes,
    No,
}

pub struct CLIListener {
    pub listener: UnixListener,
}

static ON_VALS: [&str; 6] = ["on", "true", "1", "enable", "activate", "zzz"];
static OFF_VALS: [&str; 5] = ["off", "false", "0", "disable", "deactivate"];

impl CLIListener {
    pub fn init() -> Result<Self, CLIError> {
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
                "A wired socket exists; taking ownership."
            );

            if let Err(err) = std::fs::remove_file(SOCKET_PATH) {
                eprintln!("Could not remove existing wired socket -- CLI tool will not work! Please remove {SOCKET_PATH} manually.");
                return Err(CLIError::Socket(err));
            }
        }

        let listener = UnixListener::bind(socket_path).map_err(CLIError::Socket)?;
        listener.set_nonblocking(true).map_err(CLIError::Socket)?;
        Ok(CLIListener { listener })
    }

    pub fn process_messages(&self, manager: &mut NotifyWindowManager, el: &EventLoopWindowTarget<()>) {
        // Since we're non-blocking, mostly this is just std::io::ErrorKind::WouldBlock.
        // For other errors, we should probably inform users to aide debugging.
        // I don't love the idea of spamming stderr here, however.
        match self.listener.accept() {
            Ok((socket, _addr)) => match handle_socket_message(manager, el, socket) {
                Ok(_) => (),
                Err(e) => eprintln!("Error while handling socket message: {:?}", e),
            },
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    eprintln!("{}", e);
                }
            }
        }
    }
}

// Socket stuff:
fn get_window_id(arg: &str, manager: &NotifyWindowManager) -> Result<WindowId, CLIError> {
    if arg == "latest" {
        let count = manager.layout_windows.values().flatten().count();

        if count > 0 {
            manager
                .find_window_ordered(count - 1)
                .ok_or(CLIError::NotificationNotFound)
        } else {
            Err(CLIError::NotificationNotFound)
        }
    } else if let Some(stripped) = arg.strip_prefix("id") {
        if let Ok(id) = stripped.parse::<u32>() {
            manager.find_window_nid(id).ok_or(CLIError::NotificationNotFound)
        } else {
            Err(CLIError::Parse("ID is not of type u32."))
        }
    } else if let Ok(idx) = arg.parse::<usize>() {
        manager
            .find_window_ordered(idx)
            .ok_or(CLIError::NotificationNotFound)
    } else {
        Err(CLIError::Parse(
            "Value must be one of latest, id<u32>, or <usize>.",
        ))
    }
}

pub fn handle_socket_message(
    manager: &mut NotifyWindowManager,
    el: &EventLoopWindowTarget<()>,
    stream: UnixStream,
) -> Result<(), CLIError> {
    let stream = BufReader::new(stream);
    for line in stream.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        println!("Received socket message: {}", line);
        if let Some((command, args)) = line.split_once(':') {
            match command {
                "drop" => {
                    if args == "all" {
                        manager.drop_windows();
                    } else {
                        let id = get_window_id(args, manager)?;
                        manager.drop_window_id(id);
                    }
                }
                "action" => {
                    let (notif_id, action_id) = args
                        .split_once(',')
                        .ok_or(CLIError::Parse("Malformed action request."))?;

                    let id = get_window_id(notif_id, manager)?;
                    let action = action_id
                        .parse::<usize>()
                        .map_err(|_| CLIError::Parse("Value is not of type usize."))?;
                    manager.trigger_action_idx(id, action);
                }
                "show" => {
                    if let Some(arg) = args.strip_prefix("id") {
                        let id = arg
                            .parse::<u32>()
                            .map_err(|_| CLIError::Parse("Value is not of type u32."))?;

                        // Try to find a notification with that id.
                        if let Some(n) = manager.history.pop(id) {
                            manager.new_notification(n, el);
                        } else {
                            return Err(CLIError::NotificationNotFound)
                        }
                    } else {
                        let num = args
                            .parse::<usize>()
                            .map_err(|_| CLIError::Parse("Value is not of type usize."))?;

                        for _ in 0..num {
                            if let Some(n) = manager.history.pop_back() {
                                manager.new_notification(n, el);
                            }
                        }
                    };
                }
                "dnd" => {
                    if ON_VALS.contains(&args) {
                        manager.set_dnd(true);
                    } else if OFF_VALS.contains(&args) {
                        manager.set_dnd(false);
                    }
                }
                "kill" => {
                    manager.should_exit = true;
                }
                _ => return Err(CLIError::InvalidCommand),
            }
        } else {
            return Err(CLIError::Parse("Malformed command."));
        }
    }

    Ok(())
}

// CLI stuff:
fn print_usage(opts: Options) {
    print!(
        "{}",
        opts.usage(
            "Usage:\twired [options]\n\t\
                            IDX refers to the Nth most recent notification, \
                            unless it is prefixed\n\tby 'id', in which case it \
                            refers to a notification via its ID.\n\t\
                            E.g.: `wired --drop 0` vs `wired --drop id2589`"
        )
    );
}

fn print_version() {
    println!(env!("CARGO_PKG_VERSION"));
}

fn validate_identifier(input: &str, allow_all: bool) -> Result<(), &'static str> {
    if input == "latest" || (input == "all" && allow_all) {
        return Ok(());
    }

    // We don't actually care about the values here -- this is just client side validation.
    if let Some(stripped) = input.strip_prefix("id") {
        match stripped.parse::<u32>() {
            Ok(_) => return Ok(()),
            Err(_) => return Err("Notification ID must be a valid unsigned integer."),
        }
    }

    match input.parse::<usize>() {
        Ok(_) => Ok(()),
        Err(_) => Err("Notification identifier must be either [latest], a valid \
                       notification ID (unsigned integer), or a valid notification index (usize)."),
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
    opts.optopt("z", "dnd", "enable/disable do not disturb mode", "[on|off]");
    opts.optopt("d", "drop", "drop/close a notification", "[latest|all|IDX]");
    opts.optopt(
        "a",
        "action",
        "execute a notification's action",
        "[latest|IDX]:[default|1|2|3]",
    );
    opts.optopt("s", "show", "show the last N notifications", "N");
    opts.optflag("r", "run", "run the wired daemon");
    opts.optflag("x", "kill", "kill the wired process");
    opts.optflag("v", "version", "print the version of wired and leave");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic_any(e.to_string()),
    };

    // Matching
    if matches.opt_present("h") {
        print_usage(opts);
        return Ok(ShouldRun::No);
    }

    if matches.opt_present("v") {
        print_version();
        return Ok(ShouldRun::No);
    }

    if matches.opt_present("r") {
        return Ok(ShouldRun::Yes);
    }

    // All these options use a socket.
    if matches.opt_present("d")
        || matches.opt_present("a")
        || matches.opt_present("s")
        || matches.opt_present("z")
        || matches.opt_present("x")
    {
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

        if matches.opt_present("x") {
            sock.write("kill:".as_bytes())
                .map_err(|e| e.to_string())?;
        }

        if let Some(to_drop) = matches.opt_str("d") {
            validate_identifier(to_drop.as_str(), true)?;
            sock.write(format!("drop:{}", to_drop).as_bytes())
                .map_err(|e| e.to_string())?;
        }

        if let Some(to_action) = matches.opt_str("a") {
            let (notification, action) = match to_action.split_once(':') {
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

        if let Some(on_off) = matches.opt_str("z") {
            if !(ON_VALS.contains(&on_off.as_str()) || OFF_VALS.contains(&on_off.as_str())) {
                return Err(
                    "The DND flag takes a bool argument, but I didn't recognize any.\n\
                        Allowed values are: on|off true|false 1|0 enable|disable"
                        .to_owned(),
                );
            }

            sock.write(format!("dnd:{}", on_off).as_bytes())
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
