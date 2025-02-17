use std::{
    env::args,
    fmt::Display,
    process::{Child, Command},
};

pub const REL_X: u16 = 0;
pub const REL_Y: u16 = 1;
pub const REL_WHEEL: u16 = 8;

pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

pub enum KeyButton {}

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
    pub fn new(display: String, devices: String, backend: Backend, mita: bool) -> Self {
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
            .env("DISPLAY", display.clone())
            .env(if mita { "mita" } else { "" }, if mita { "1" } else { "" })
            .spawn()
            .unwrap();
        let pid = proc.id();

        Self {
            pid,
            proc,
            devices,
            display,
            backend,
        }
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
