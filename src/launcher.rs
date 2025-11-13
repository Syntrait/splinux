// ENV_VARS gamescope -W 1920 -H 1080 --backend sdl -- bwrap --dev-bind / / --bind ~/second-player ~/saves [RUN GAME]
//
// ENV_VARS
// width
// height
// backend?
// bwrap dirbinds
// game arguments

use std::process::Command;

use crate::types::{CommandType, WindowGeometry};

pub fn construct_command(geometry: WindowGeometry, command: &CommandType) -> Command {
    let mut cmd = Command::new("gamescope");

    // gamescope arguments
    cmd.env("ENABLE_GAMESCOPE_WSI", "0");
    cmd.arg("-W");
    cmd.arg(geometry.width.to_string());
    cmd.arg("-H");
    cmd.arg(geometry.height.to_string());
    cmd.arg("--backend");
    cmd.arg("sdl"); // TODO: make this configurable later on
    cmd.arg("--");

    // bwrap arguments
    cmd.arg("bwrap");
    cmd.arg("--dev-bind");
    cmd.arg("/");
    cmd.arg("/");
    // TODO: bind arguments

    // game arguments
    match command {
        CommandType::SteamLaunch { appid } => {}
        CommandType::Manual { command } => {
            cmd.arg(command);
            println!("executablepath: {}", command);
        }
    }

    cmd
}
