// TODO: Rename this file to native_backend.rs

use crate::types::{
    BTN_EAST, BTN_MODE, BTN_NORTH, BTN_SELECT, BTN_SOUTH, BTN_START, BTN_THUMBL, BTN_THUMBR,
    BTN_TL, BTN_TL2, BTN_TR, BTN_TR2, BTN_WEST, BackendCommand, ClientError, Device, DeviceType,
    get_devices,
};
use anyhow::{Context, Result};
use evdev::{AbsoluteAxisCode, Device as EvdevDevice, RelativeAxisCode};
use flume::{Receiver, TryRecvError};
#[cfg(feature = "xtst")]
use libc::{c_char, c_int, c_ulong};
use std::{
    thread::{JoinHandle, spawn},
    u16,
};
use uinput::{
    Device as UInputDevice,
    event::{self, Controller, controller::GamePad},
};
use x11rb::{
    connection::Connection, protocol::xtest::ConnectionExt, rust_connection::RustConnection,
};

#[cfg(feature = "xtst")]
#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *mut c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
}

#[cfg(feature = "xtst")]
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

#[cfg(feature = "xtst")]
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

#[cfg(feature = "xtst")]
struct DisplayPtr(*mut Display);

#[cfg(feature = "xtst")]
unsafe impl Send for DisplayPtr {}

#[cfg(feature = "xtst")]
impl Clone for DisplayPtr {
    fn clone(&self) -> Self {
        let mut null: c_char = 0;
        Self::new(&mut null)
    }
}

#[cfg(feature = "xtst")]
impl Copy for DisplayPtr {}

#[cfg(feature = "xtst")]
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
    fn new(path: String, suffix: String, displayvar: &String) -> Result<Self> {
        let evdev_device = EvdevDevice::open(&path)?;
        let (x11connection, _) = RustConnection::connect(Some(displayvar))?;

        let displayvar = displayvar.clone();

        Ok(Self {
            path,
            suffix,
            evdev_device,
            uinput_device: None,
            devicetype: DeviceType::Unknown,
            configured: false,
            display: None,
            x11connection,
            displayvar,
        })
    }

    fn configure(&mut self) -> Result<()> {
        self.configured = true;
        if self.devicetype == DeviceType::Gamepad {
            self.uinput_device = Some(
                uinput::default()
                    .with_context(|| "initializing uinput device")?
                    .name(format!("Splinux Virtual Gamepad Device {}", self.suffix))?
                    .event(event::Controller::All)?
                    .create()?,
            );
        } else if self.devicetype == DeviceType::Mouse {
            #[cfg(feature = "xtst")]
            {
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
                            #[cfg(feature = "xtst")]
                            {
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
                            #[cfg(not(feature = "xtst"))]
                            {
                                // absolute cursor only
                                conn.warp_pointer(0, window, 0, 0, 0, 0, value as i16, 0)?;

                                conn.flush()?;
                            }
                        }
                        RelativeAxisCode::REL_Y => {
                            self.devicetype = DeviceType::Mouse;
                            if !self.configured {
                                continue;
                            }
                            #[cfg(feature = "xtst")]
                            {
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

                            #[cfg(not(feature = "xtst"))]
                            {
                                // absolute cursor only
                                conn.warp_pointer(0, window, 0, 0, 0, 0, 0, value as i16)?;

                                conn.flush()?;
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

pub fn client(devices: String, displayvar: String, rx: Receiver<BackendCommand>) {
    let dev_nums: Vec<&str> = devices.split(",").collect();
    let mut handles: Vec<JoinHandle<()>> = vec![];

    for device_num in dev_nums {
        let path = format!("/dev/input/event{}", device_num);
        let mut perip = Peripheral::new(path, device_num.to_owned(), &displayvar).unwrap();
        let rx = rx.clone();

        let handle = spawn(move || {
            perip.run(rx).unwrap();
        });

        handles.push(handle);
    }

    for handle in handles {
        // if one thread fails, the other should too.
        // this prevents lockups, because disconnecting one device frees the other (perhaps unpluggable) device
        handle.join().unwrap();
    }
}
