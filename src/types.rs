use flume::Sender;
use std::{
    env::args,
    fmt::Display,
    process::{Child, Command},
};

pub const REL_X: u16 = 0;
pub const REL_Y: u16 = 1;
pub const REL_WHEEL: u16 = 8;

pub enum StateCommand {
    ToggleFPSMode,
    GetFPSMode(Sender<bool>),
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
    Legacy,
}

impl Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Enigo => write!(f, "Enigo"),
            Backend::Legacy => write!(f, "Legacy"),
        }
    }
}

impl Client {
    pub fn new(display: String, devices: String, backend: Backend) -> Result<Self, String> {
        if display.contains("-") && backend == Backend::Legacy {
            return Err("Legacy backend doesn't support Wayland".to_owned());
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
                Backend::Legacy => [
                    "run",
                    "-d",
                    display.as_str(),
                    "-i",
                    devices.as_str(),
                    "-b",
                    "legacy",
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
