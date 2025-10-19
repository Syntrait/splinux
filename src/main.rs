mod gui;
mod native_backend;
mod types;

use clap::{Arg, ArgAction, Command};
use gui::start;
use native_backend::backend as nativebackend;
use std::env::args;
use types::Backend;

fn main() {
    let args1: Vec<String> = args().collect();
    if args1.len() == 1 {
        start();
        return;
    }

    let matches = Command::new("splinux")
        .about("split-screen solution for linux")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
        .subcommand(Command::new("gui").long_flag("gui").about("show the gui"))
        .subcommand(
            Command::new("run")
                .long_flag("run")
                .about("convert device input to display input")
                .arg(
                    Arg::new("display")
                        .short('d')
                        .long("display")
                        .help("the display id, could be something like \"wayland-2\" or \":30\"")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .help("input device ids, could be something like \"25,28\"")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("gui", _)) => {
            start();
        }
        Some(("run", x)) => {
            let display: &String = x.get_one("display").unwrap();
            let input: &String = x.get_one("input").unwrap();

            // TODO: headless support
            // TODO: figure out and make this work
            //nativeclient(input.to_owned(), "".to_owned());
        }
        _ => unreachable!(),
    }
}
