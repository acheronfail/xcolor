mod atoms;
mod cli;
mod color;
mod draw;
mod format;
mod location;
mod pixel;
mod selection;
mod util;

use clap::ArgMatches;
use failure::{err_msg, Error};
use nix::unistd::ForkResult;
use xcb::base::Connection;

use crate::cli::get_cli;
use crate::format::{Format, FormatColor, FormatString};
use crate::location::wait_for_location;
use crate::selection::{into_daemon, set_selection, Selection};

fn run(args: &ArgMatches) -> Result<(), Error> {
    fn error(message: &str) -> ! {
        clap::Error::with_description(message, clap::ErrorKind::InvalidValue).exit()
    }

    let custom_format;
    let simple_format;
    let formatter: &dyn FormatColor = if let Some(custom) = args.value_of("custom") {
        custom_format = custom
            .parse::<FormatString>()
            .unwrap_or_else(|_| error("Invalid format string"));
        &custom_format
    } else {
        simple_format = args
            .value_of("format")
            .unwrap_or("hex")
            .parse::<Format>()
            .unwrap_or_else(|e| error(&format!("{}", e)));
        &simple_format
    };

    let scale = args
        .value_of("scale")
        .unwrap_or("8")
        .parse::<u32>()
        .unwrap_or_else(|e| error(&format!("Invalid scale value: {}", e)));
    let preview_size = args
        .value_of("preview_size")
        .unwrap_or("255")
        .parse::<u32>()
        .unwrap_or_else(|e| error(&format!("Invalid preview_size value: {}", e)));

    let selection = args.values_of("selection").and_then(|mut v| {
        v.next()
            .map_or(Some(Selection::Clipboard), |v| v.parse::<Selection>().ok())
    });
    let use_selection = selection.is_some();
    let background = std::env::var("XCOLOR_FOREGROUND").is_err();

    let mut in_parent = true;

    let (conn, screen) = Connection::connect_with_xlib_display()?;

    {
        let screen = conn
            .get_setup()
            .roots()
            .nth(screen as usize)
            .ok_or_else(|| err_msg("Could not find screen"))?;
        let root = screen.root();

        if let Some(color) = wait_for_location(&conn, &screen, preview_size, scale)? {
            let output = formatter.format(color);

            if use_selection {
                if background {
                    in_parent = match into_daemon()? {
                        ForkResult::Parent { .. } => true,
                        ForkResult::Child => false,
                    }
                }

                if !(background && in_parent) {
                    set_selection(&conn, root, &selection.unwrap(), &output)?;
                }
            } else {
                println!("{}", output);
            }
        }
    }

    if background && in_parent {
        std::mem::forget(conn);
    }

    Ok(())
}

fn main() {
    let args = get_cli().get_matches();
    if let Err(err) = run(&args) {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
