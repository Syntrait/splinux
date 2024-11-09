mod client;
mod gui;
mod types;

use client::client;
use gui::start;
use std::env::args;

// Xephyr -softCursor -screen 1920x540 :99

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() == 1 {
        start();
    } else if args.len() == 3 && args[1] == "-client" {
        client(args[2].clone());
    }
}
