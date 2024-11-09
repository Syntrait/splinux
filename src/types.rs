use std::{
    env::args,
    process::{Child, Command},
};

// CODE
pub const REL_X: u16 = 0;
pub const REL_Y: u16 = 1;
pub const REL_WHEEL: u16 = 8;

pub struct Client {
    pub pid: u32,
    proc: Child,
    pub devices: String,
    pub display: String,
}

impl Client {
    pub fn new(display: String, devices: String) -> Self {
        let args: Vec<String> = args().collect();
        let proc = Command::new(args[0].clone())
            .args(["-client", devices.as_str()])
            .env("DISPLAY", display.clone())
            .spawn()
            .unwrap();
        let pid = proc.id();

        Self {
            pid,
            proc,
            devices,
            display,
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
