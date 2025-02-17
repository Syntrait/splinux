mod client;
mod gui;
mod legacy_client;
mod types;

use clap::{Arg, ArgAction, Command};
use client::client;
use gui::start;
use legacy_client::client as legacy_client;
use std::env::{args, var_os};
use types::Backend;

/*
- Native Wayland is now supported
- Gamescope is not a hard dependency anymore
- Input sending is now handled by Enigo
- Proper command line argument parsing
- Support for multiple gamepads


*/

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
                )
                .arg(
                    Arg::new("backend")
                        .short('b')
                        .long("backend")
                        .help("the input sender backend, \"enigo\" or \"legacy\". default is enigo")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .default_value("enigo"),
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
            let backend: Backend = if x
                .get_one("backend")
                .is_some_and(|x: &String| x.to_lowercase() == "legacy")
            {
                Backend::Legacy
            } else {
                Backend::Enigo
            };

            match backend {
                Backend::Enigo => {
                    client(
                        input.to_owned(),
                        display.to_owned(),
                        var_os("mita").is_some_and(|x| x == "1"),
                    );
                }
                Backend::Legacy => {
                    legacy_client(input.to_owned());
                }
            }
        }
        _ => unreachable!(),
    }
    /*

    splinux

    splinux gui

    splinux --help

    splinux run -d wayland-2 -i 23,24

    splinux run -d :11 -i 23,24

    splinux run -d wayland-2 -i 23,24 -b legacy

    */
}
