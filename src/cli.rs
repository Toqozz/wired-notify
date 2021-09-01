use std::io::Write;
use std::os::unix::net::UnixStream;
use std::panic::panic_any;

use getopts::Options;

pub enum ShouldRun {
    Yes,
    No,
}

fn print_usage(opts: Options) {
    print!("{}", opts.usage("Usage:\twired [options]\n\tIDX refers to the Nth most recent notification."));
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
        let mut sock = match UnixStream::connect(crate::SOCKET_PATH) {
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
