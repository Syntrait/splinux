use std::{
    env::args,
    fmt::Display,
    process::{Child, Command},
};

use thiserror::Error;

pub const BTN_SOUTH: u16 = 304; // A
pub const BTN_EAST: u16 = 305; // B
pub const BTN_NORTH: u16 = 307; // X
pub const BTN_WEST: u16 = 308; // Y

pub const BTN_TL: u16 = 310; // LB
pub const BTN_TR: u16 = 311; // RB

pub const BTN_TL2: u16 = 312; // LT
pub const BTN_TR2: u16 = 313; // RT

pub const BTN_THUMBL: u16 = 317; // LS
pub const BTN_THUMBR: u16 = 318; // RS

#[derive(PartialEq)]
pub enum DeviceType {
    Keyboard,
    Mouse,
    Gamepad,
    Unknown,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Couldn't open the X11 DISPLAY")]
    X11DisplayOpenError,
    #[error("Relative mouse movement failed")]
    RelativeMovementFail,
}

pub struct Client {
    pub pid: u32,
    proc: Child,
    pub devices: String,
    pub display: String,
    pub backend: Backend,
}

#[derive(PartialEq, Clone)]
pub enum Backend {
    Enigo,
    Native,
}

impl Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Enigo => write!(f, "Enigo"),
            Backend::Native => write!(f, "Native"),
        }
    }
}

impl Client {
    pub fn new(display: String, devices: String, backend: Backend) -> Result<Self, String> {
        if display.contains("-") && backend == Backend::Native {
            return Err("Native backend doesn't support Wayland".to_owned());
        }

        let args: Vec<String> = args().collect();
        let proc = Command::new(args[0].clone())
            .args(match backend {
                Backend::Enigo => [
                    "run",
                    "-d",
                    display.as_str(),
                    "-i",
                    devices.as_str(),
                    "-b",
                    "enigo",
                ],
                Backend::Native => [
                    "run",
                    "-d",
                    display.as_str(),
                    "-i",
                    devices.as_str(),
                    "-b",
                    "native",
                ],
            })
            .env(
                if display.contains(":") {
                    "DISPLAY"
                } else {
                    "WAYLAND_DISPLAY"
                },
                if display.contains(":") {
                    display.as_str()
                } else {
                    display.as_str()
                },
            )
            .spawn()
            .unwrap();
        let pid = proc.id();

        Ok(Self {
            pid,
            proc,
            devices,
            display,
            backend,
        })
    }

    pub fn is_alive(&mut self) -> bool {
        match self.proc.try_wait() {
            Ok(Some(_)) => return false,
            Ok(None) => return true,
            Err(x) => {
                panic!("{}", x);
            }
        }
    }

    pub fn kill(&mut self) {
        self.proc.kill().unwrap();
    }
}
