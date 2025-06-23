use std::{
    collections::HashMap,
    env::args,
    fmt::Display,
    fs::read_dir,
    os::unix::fs::FileTypeExt,
    process::{Child, Command},
};

use anyhow::Result;
use evdev::{Device as EvdevDevice, EventType, KeyCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Gamepad
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

pub const BTN_SELECT: u16 = 314; // Select
pub const BTN_START: u16 = 315; // Start
pub const BTN_MODE: u16 = 316; // Guide/PS Button

#[derive(PartialEq, Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "type")]
pub enum DeviceType {
    Keyboard,
    Mouse,
    Gamepad,
    Unknown,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Couldn't spawn the client process")]
    SpawnError,
    #[error("Unsupported")]
    UnsupportedError,
    #[error("Couldn't open the X11 DISPLAY")]
    X11DisplayOpenError,
    #[error("Relative mouse movement failed")]
    RelativeMovementFail,
}

// used by gui.rs

#[derive(Serialize, Deserialize)]
pub struct Client {
    pub name: String,
    pub pid: u32,
    #[serde(skip_serializing, skip_deserializing)]
    proc: Option<Child>,
    pub devices: Vec<Device>,
    pub display: String,
    pub backend: Backend,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            name: self.name.to_owned(),
            pid: self.pid,
            proc: None,
            devices: self.devices.clone(),
            display: self.display.to_owned(),
            backend: self.backend,
        }
    }
}

pub enum GuiState {
    MainMenu,     // Select a preset
    EditPreset,   // Create & Edit preset
    ManagePreset, // Manage the preset (start/stop individual clients, restart, switch to edit mode buttton)
    EditClient,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Device {
    name: String,
    index: u16,
    showindex: bool,
    pub devicetype: DeviceType,
    // dont depend on this too much
    pub namenum: Option<u16>,
}

impl Device {
    pub fn new(name: String, index: u16, devicetype: DeviceType, namenum: Option<u16>) -> Self {
        Self {
            name,
            index,
            showindex: false,
            devicetype,
            namenum,
        }
    }

    pub fn get_name(&self) -> String {
        if self.showindex {
            format!("{} {}", self.name, self.index + 1)
        } else {
            self.name.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Preset {
    pub name: String,
    pub clients: Vec<Client>,
}

impl Preset {
    pub fn new(name: String, clients: Vec<Client>) -> Self {
        Self { name, clients }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeviceList {
    pub devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum Backend {
    Enigo,
    Native,
}

impl DeviceList {
    pub fn new(devices: Vec<Device>) -> Self {
        DeviceList { devices }
    }
}

pub fn get_devices() -> Vec<Device> {
    let mut devices: Vec<Device> = vec![];

    for device in read_dir("/dev/input").unwrap() {
        if let Ok(dev) = device {
            if !dev.file_type().unwrap().is_char_device() {
                continue;
            }
            let filename = dev.file_name().into_string().unwrap();
            if !filename.starts_with("event") {
                continue;
            }
            let path = "/dev/input/".to_owned() + &filename;
            let evdev_device = EvdevDevice::open(&path).unwrap();

            let supports = evdev_device.supported_events();

            if !supports.contains(EventType::ABSOLUTE)
                && !supports.contains(EventType::FORCEFEEDBACK)
                && !supports.contains(EventType::KEY)
                && !supports.contains(EventType::RELATIVE)
            {
                continue;
            }

            let is_gamepad = evdev_device
                .supported_keys()
                .map_or(false, |keys| keys.contains(KeyCode::BTN_SOUTH));

            let is_mouse = evdev_device
                .supported_keys()
                .map_or(false, |keys| keys.contains(KeyCode::BTN_LEFT));

            let is_keyboard = evdev_device
                .supported_keys()
                .map_or(false, |keys| keys.contains(KeyCode::KEY_ENTER));

            let count = [is_gamepad, is_mouse, is_keyboard]
                .iter()
                .filter(|&&x| x)
                .count();

            let devtype = match count {
                0 => {
                    continue;
                }
                1 => {
                    if is_gamepad {
                        DeviceType::Gamepad
                    } else if is_mouse {
                        DeviceType::Mouse
                    } else {
                        DeviceType::Keyboard
                    }
                }
                _ => DeviceType::Unknown,
            };

            let name = evdev_device.name().unwrap();
            let namenum: u16 = filename.replacen("event", "", 1).parse().unwrap();

            let newdev = Device::new(name.to_owned(), 0, devtype, Some(namenum));

            devices.push(newdev);
        }
    }

    // devices connected first are more likely to be relevant
    devices.sort_by_key(|dev| dev.namenum.unwrap());

    // handle duplicate named devices with index numbers
    let mut store: HashMap<String, u16> = HashMap::new();

    for perip in devices.iter_mut() {
        if let Some(val) = store.get_mut(&perip.get_name()) {
            *val += 1;
            perip.index = *val;
            perip.showindex = true;
        } else {
            store.insert(perip.get_name(), 0);
        }
    }

    devices
}

impl Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Native => write!(f, "Native"),
            Backend::Enigo => write!(f, "Enigo"),
        }
    }
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Gamepad => write!(f, "Gamepad"),
            DeviceType::Mouse => write!(f, "Mouse"),
            DeviceType::Keyboard => write!(f, "Keyboard"),
            DeviceType::Unknown => write!(f, "Unknown"),
        }
    }
}

// used by gui.rs
// launch a subprocess
impl Client {
    pub fn new(
        name: String,
        display: String,
        devices: &Vec<Device>,
        backend: Backend,
    ) -> Result<Self> {
        if display.contains("-") && backend == Backend::Native {
            return Err(ClientError::UnsupportedError)?;
        }

        let args: Vec<String> = args().collect();
        let proc = Command::new(args[0].clone())
            .args(match backend {
                Backend::Native => [
                    "run",
                    "-d",
                    display.as_str(),
                    "-i",
                    "TODO: devices",
                    "-b",
                    "native",
                ],
                Backend::Enigo => [
                    "run",
                    "-d",
                    display.as_str(),
                    "-i",
                    "TODO: devices",
                    "-b",
                    "enigo",
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
            .spawn()?;
        let pid = proc.id();
        let devices = devices.clone();

        Ok(Self {
            name,
            pid,
            proc: Some(proc),
            devices,
            display,
            backend,
        })
    }

    pub fn is_alive(&mut self) -> bool {
        if let Some(proc) = self.proc.as_mut() {
            match proc.try_wait() {
                Ok(Some(_)) => return false,
                Ok(None) => return true,
                Err(x) => {
                    panic!("{}", x);
                }
            }
        } else {
            return false;
        }
    }

    pub fn kill(&mut self) {
        if self.is_alive() {
            self.proc.as_mut().unwrap().kill().unwrap();
        }
    }
}
