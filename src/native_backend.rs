use crate::types::{
    BTN_EAST, BTN_MODE, BTN_NORTH, BTN_SELECT, BTN_SOUTH, BTN_START, BTN_THUMBL, BTN_THUMBR,
    BTN_TL, BTN_TL2, BTN_TR, BTN_TR2, BTN_WEST, BackendCommand, ClientError, Device, DeviceType,
    fix_namenums,
};
use anyhow::{Context, Result, anyhow};
use evdev::{AbsoluteAxisCode, Device as EvdevDevice, RelativeAxisCode};
use flume::{Receiver, TryRecvError};
use libc::{c_char, c_int, c_ulong};
use std::{
    collections::HashMap,
    fs,
    thread::{JoinHandle, spawn},
};
use uinput::{
    Device as UInputDevice,
    event::{self, Controller, controller::GamePad},
};
use x11rb::{
    connection::Connection, protocol::xtest::ConnectionExt, rust_connection::RustConnection,
};

/// Holds pre-created virtual gamepad devices and their /dev/input paths.
/// Created before game launch so bwrap can isolate device access.
pub struct PrecreatedGamepads {
    /// (virtual_event_num, virtual_js_num) for bwrap device isolation
    info: Vec<(u16, Option<u16>)>,
    /// physical_event_num → pre-created UInputDevice
    gamepads: HashMap<u16, UInputDevice>,
}

impl PrecreatedGamepads {
    /// Returns the virtual device info needed by the launcher for bwrap isolation.
    pub fn virtual_device_info(&self) -> &[(u16, Option<u16>)] {
        &self.info
    }
}

/// Pre-create virtual gamepad devices for all gamepad-type devices in the list.
/// Must be called before launching the game so bwrap can restrict /dev/input access.
pub fn precreate_virtual_gamepads(devices: &mut Vec<Device>) -> Result<PrecreatedGamepads> {
    fix_namenums(devices)?;

    let mut info = Vec::new();
    let mut gamepads = HashMap::new();

    for device in devices.iter() {
        if device.devicetype != DeviceType::Gamepad {
            continue;
        }
        if let Some(devid) = device.namenum {
            let suffix = devid.to_string();
            let name = format!("Splinux Virtual Gamepad Device {}", suffix);

            let uinput_dev = create_virtual_gamepad(&suffix)?;

            // Wait briefly for the kernel to create the device nodes
            std::thread::sleep(std::time::Duration::from_millis(50));

            let (event_num, js_num) = find_virtual_device_info(&name)
                .ok_or_else(|| anyhow!("Could not find virtual device node for '{}'", name))?;

            println!(
                "Pre-created virtual gamepad '{}': event{}, js{:?}",
                name, event_num, js_num
            );

            info.push((event_num, js_num));
            gamepads.insert(devid, uinput_dev);
        }
    }

    Ok(PrecreatedGamepads { info, gamepads })
}

/// Create a virtual gamepad with proper button AND axis registration.
fn create_virtual_gamepad(suffix: &str) -> Result<UInputDevice> {
    use event::absolute::{Hat, Position};

    let dev = uinput::default()
        .with_context(|| "initializing uinput device")?
        .name(format!("Splinux Virtual Gamepad Device {}", suffix))?
        // All gamepad/controller buttons
        .event(event::Controller::All)?
        // Left stick
        .event(event::Absolute::Position(Position::X))?
        .min(-32768)
        .max(32767)
        .fuzz(16)
        .flat(128)
        .event(event::Absolute::Position(Position::Y))?
        .min(-32768)
        .max(32767)
        .fuzz(16)
        .flat(128)
        // Right stick
        .event(event::Absolute::Position(Position::RX))?
        .min(-32768)
        .max(32767)
        .fuzz(16)
        .flat(128)
        .event(event::Absolute::Position(Position::RY))?
        .min(-32768)
        .max(32767)
        .fuzz(16)
        .flat(128)
        // Left trigger
        .event(event::Absolute::Position(Position::Z))?
        .min(0)
        .max(1023)
        // Right trigger
        .event(event::Absolute::Position(Position::RZ))?
        .min(0)
        .max(1023)
        // D-pad
        .event(event::Absolute::Hat(Hat::X0))?
        .min(-1)
        .max(1)
        .event(event::Absolute::Hat(Hat::Y0))?
        .min(-1)
        .max(1)
        .create()?;

    Ok(dev)
}

/// Find the event number and js number of a virtual input device by name.
fn find_virtual_device_info(name: &str) -> Option<(u16, Option<u16>)> {
    for entry in fs::read_dir("/sys/devices/virtual/input").ok()? {
        let entry = entry.ok()?;
        if !entry
            .file_name()
            .to_str()?
            .starts_with("input")
        {
            continue;
        }
        let dev_name = fs::read_to_string(entry.path().join("name")).ok()?;
        if dev_name.trim() != name {
            continue;
        }
        let mut event_num = None;
        let mut js_num = None;
        for sub in fs::read_dir(entry.path()).ok()? {
            let sub = sub.ok()?;
            let fname = sub.file_name().into_string().ok()?;
            if let Some(num) = fname.strip_prefix("event") {
                event_num = num.parse().ok();
            } else if let Some(num) = fname.strip_prefix("js") {
                js_num = num.parse().ok();
            }
        }
        if let Some(ev) = event_num {
            return Some((ev, js_num));
        }
    }
    None
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *mut c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
}

#[link(name = "Xtst")]
unsafe extern "C" {
    fn XTestFakeRelativeMotionEvent(
        display: *mut Display,
        x: c_int,
        y: c_int,
        time: c_ulong,
    ) -> c_int;
    fn XFlush(display: *mut Display);
}

#[repr(C)]
struct Display {
    // silences warnings
    _private: *mut (),
}

struct Peripheral {
    path: String,
    suffix: String,
    evdev_device: EvdevDevice,
    uinput_device: Option<UInputDevice>,
    devicetype: DeviceType,
    configured: bool,
    display: Option<DisplayPtr>,
    x11connection: RustConnection,
    displayvar: String,
}

impl Drop for Peripheral {
    fn drop(&mut self) {
        let _ = self.evdev_device.ungrab();

        if let Some(display) = self.display {
            unsafe {
                XCloseDisplay(display.0);
            }
        }
    }
}

struct DisplayPtr(*mut Display);

unsafe impl Send for DisplayPtr {}

impl Clone for DisplayPtr {
    fn clone(&self) -> Self {
        let mut null: c_char = 0;
        Self::new(&mut null)
    }
}

impl Copy for DisplayPtr {}

impl DisplayPtr {
    fn new(display: *mut i8) -> Self {
        unsafe {
            Self {
                0: XOpenDisplay(display),
            }
        }
    }
}

impl Peripheral {
    fn new(
        path: String,
        suffix: String,
        displayvar: &String,
        precreated_uinput: Option<UInputDevice>,
    ) -> Result<Self> {
        let evdev_device = EvdevDevice::open(&path)?;
        let (x11connection, _) = RustConnection::connect(Some(displayvar))?;

        let displayvar = displayvar.clone();

        // If a pre-created uinput device was provided, mark as a configured gamepad
        let (uinput_device, configured, devicetype) = if let Some(dev) = precreated_uinput {
            (Some(dev), true, DeviceType::Gamepad)
        } else {
            (None, false, DeviceType::Unknown)
        };

        Ok(Self {
            path,
            suffix,
            evdev_device,
            uinput_device,
            devicetype,
            configured,
            display: None,
            x11connection,
            displayvar,
        })
    }

    fn configure(&mut self) -> Result<()> {
        self.configured = true;
        if self.devicetype == DeviceType::Gamepad {
            self.uinput_device = Some(create_virtual_gamepad(&self.suffix)?);
        } else if self.devicetype == DeviceType::Mouse {
            let mut displayparse: c_char = self.displayvar.parse().unwrap();
            self.display = Some(DisplayPtr::new(&mut displayparse));
            if let Some(display) = self.display.as_mut() {
                if display.0.is_null() {
                    return Err(ClientError::X11DisplayOpenError)?;
                }
            } else {
                return Err(ClientError::X11DisplayOpenError)?;
            }
        }
        println!("Configuration completed for {}", self.path);
        Ok(())
    }

    fn run(&mut self, rx: Receiver<BackendCommand>) -> Result<()> {
        if let Err(x) = self.evdev_device.grab() {
            eprintln!("Couldn't grab the input device, continuing anyways.");
            eprintln!("{}: {}", x.kind(), x.to_string());
        }

        loop {
            match rx.try_recv() {
                Err(TryRecvError::Empty) => {}
                Ok(BackendCommand::Terminate) => {
                    return Ok(());
                }
                Ok(BackendCommand::PauseGrab) => {
                    let _ = self.evdev_device.ungrab();
                }
                Ok(BackendCommand::UnpauseGrab) => {
                    let _ = self.evdev_device.grab();
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(TryRecvError::Disconnected)?;
                }
            }
            if !self.configured && self.devicetype != DeviceType::Unknown {
                self.configure()?;
            }
            for e in self.evdev_device.fetch_events()? {
                match e.destructure() {
                    evdev::EventSummary::Key(_, keycode, keystate) => {
                        match keycode.0 {
                            mbutton @ 272..=274 => {
                                self.devicetype = DeviceType::Mouse;
                                self.x11connection.xtest_fake_input(
                                    if keystate == 1 {
                                        x11rb::protocol::xproto::BUTTON_PRESS_EVENT
                                    } else {
                                        x11rb::protocol::xproto::BUTTON_RELEASE_EVENT
                                    },
                                    match mbutton {
                                        272 => 1, // LMB
                                        273 => 3, // RMB
                                        274 => 2, // MMB
                                        _ => unreachable!(),
                                    },
                                    0,
                                    0,
                                    0,
                                    0,
                                    0,
                                )?;

                                self.x11connection.flush()?;
                            }
                            gamepadbutton @ 304..=318 => {
                                self.devicetype = DeviceType::Gamepad;
                                if !self.configured {
                                    continue;
                                };
                                if let Some(dev) = self.uinput_device.as_mut() {
                                    dev.send(
                                        libinput_key_to_uinput_event(gamepadbutton),
                                        keystate,
                                    )?;

                                    dev.synchronize()?;
                                }
                            }
                            keyid => {
                                if keystate == 2 {
                                    // constant hold events, we only care about start and stop
                                    continue;
                                }
                                self.devicetype = DeviceType::Keyboard;
                                self.x11connection.xtest_fake_input(
                                    if keystate == 1 {
                                        x11rb::protocol::xproto::KEY_PRESS_EVENT
                                    } else {
                                        x11rb::protocol::xproto::KEY_RELEASE_EVENT
                                    },
                                    // if you add +8 to libinput key id, you get x11 key id for the corresponding key
                                    keyid as u8 + 8,
                                    0,
                                    0,
                                    0,
                                    0,
                                    0,
                                )?;

                                self.x11connection.flush()?;
                            }
                        }
                    }
                    evdev::EventSummary::RelativeAxis(_, code, value) => match code {
                        RelativeAxisCode::REL_X => {
                            self.devicetype = DeviceType::Mouse;
                            if !self.configured {
                                continue;
                            }
                            if let Some(display) = self.display.as_mut() {
                                unsafe {
                                    let display = display.to_owned();
                                    let success =
                                        XTestFakeRelativeMotionEvent(display.0, value, 0, 0);
                                    if success == 0 {
                                        return Err(ClientError::RelativeMovementFail)?;
                                    }
                                    XFlush(display.0);
                                }
                            }
                        }
                        RelativeAxisCode::REL_Y => {
                            self.devicetype = DeviceType::Mouse;
                            if !self.configured {
                                continue;
                            }
                            if let Some(display) = self.display.as_mut() {
                                unsafe {
                                    let display = display.to_owned();
                                    let success =
                                        XTestFakeRelativeMotionEvent(display.0, 0, value, 0);
                                    if success == 0 {
                                        return Err(ClientError::RelativeMovementFail)?;
                                    }
                                    XFlush(display.0);
                                }
                            }
                        }
                        RelativeAxisCode::REL_WHEEL => {
                            self.devicetype = DeviceType::Mouse;
                            self.x11connection.xtest_fake_input(
                                x11rb::protocol::xproto::BUTTON_PRESS_EVENT,
                                if value == 1 { 4 } else { 5 },
                                0,
                                0,
                                0,
                                0,
                                0,
                            )?;

                            self.x11connection.flush()?;
                        }

                        _ => {}
                    },
                    evdev::EventSummary::AbsoluteAxis(_, code, value) => match code {
                        AbsoluteAxisCode::ABS_X => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Position(event::absolute::Position::X),
                                    value,
                                )?;

                                dev.synchronize()?;
                            }
                        }
                        AbsoluteAxisCode::ABS_Y => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Position(event::absolute::Position::Y),
                                    value,
                                )?;

                                dev.synchronize()?;
                            }
                        }

                        AbsoluteAxisCode::ABS_RX => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Position(event::absolute::Position::RX),
                                    value,
                                )?;

                                dev.synchronize()?;
                            }
                        }
                        AbsoluteAxisCode::ABS_RY => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Position(event::absolute::Position::RY),
                                    value,
                                )?;

                                dev.synchronize()?;
                            }
                        }

                        // Left trigger
                        AbsoluteAxisCode::ABS_Z => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Position(event::absolute::Position::Z),
                                    value,
                                )?;
                                dev.synchronize()?;
                            }
                        }
                        // Right trigger
                        AbsoluteAxisCode::ABS_RZ => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Position(event::absolute::Position::RZ),
                                    value,
                                )?;
                                dev.synchronize()?;
                            }
                        }
                        // D-pad
                        AbsoluteAxisCode::ABS_HAT0X => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Hat(event::absolute::Hat::X0),
                                    value,
                                )?;
                                dev.synchronize()?;
                            }
                        }
                        AbsoluteAxisCode::ABS_HAT0Y => {
                            self.devicetype = DeviceType::Gamepad;
                            if !self.configured {
                                continue;
                            };
                            if let Some(dev) = self.uinput_device.as_mut() {
                                dev.send(
                                    event::Absolute::Hat(event::absolute::Hat::Y0),
                                    value,
                                )?;
                                dev.synchronize()?;
                            }
                        }

                        _ => {}
                    },

                    _ => {}
                }
            }
        }
    }
}

fn libinput_key_to_uinput_event(keyid: u16) -> uinput::Event {
    uinput::Event::Controller(Controller::GamePad(match keyid {
        BTN_SOUTH => GamePad::A,
        BTN_EAST => GamePad::B,
        BTN_NORTH => GamePad::X,
        BTN_WEST => GamePad::Y,

        BTN_TL => GamePad::TL,
        BTN_TR => GamePad::TR,

        BTN_TL2 => GamePad::TL2,
        BTN_TR2 => GamePad::TR2,

        BTN_THUMBL => GamePad::ThumbL,
        BTN_THUMBR => GamePad::ThumbR,

        BTN_SELECT => GamePad::Select,
        BTN_START => GamePad::Start,
        BTN_MODE => GamePad::Mode,

        _ => GamePad::Mode,
    }))
}

pub fn backend(
    mut devices: Vec<Device>,
    displayvar: String,
    rx: Receiver<BackendCommand>,
    mut precreated: PrecreatedGamepads,
) -> Result<()> {
    let mut handles: Vec<JoinHandle<()>> = vec![];
    fix_namenums(&mut devices)?;

    for device in devices {
        if let Some(devid) = device.namenum {
            let path = format!("/dev/input/event{}", devid);
            let precreated_uinput = precreated.gamepads.remove(&devid);
            let mut perip =
                Peripheral::new(path, devid.to_string(), &displayvar, precreated_uinput)?;
            let rx = rx.clone();

            let handle = spawn(move || {
                perip.run(rx).unwrap();
            });

            handles.push(handle);
        }
    }

    for handle in handles {
        // if one thread fails, the other should too.
        // this prevents lockups, because disconnecting one device frees the other (perhaps unpluggable) device
        handle.join().unwrap();
    }
    Ok(())
}
