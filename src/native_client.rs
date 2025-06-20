use crate::types::{
    ClientError, DeviceType, BTN_EAST, BTN_NORTH, BTN_SOUTH, BTN_THUMBL, BTN_THUMBR, BTN_TL,
    BTN_TL2, BTN_TR, BTN_TR2, BTN_WEST,
};
use anyhow::Result;
use evdev::{Device as EvdevDevice, RelativeAxisCode};
#[cfg(feature = "xtst")]
use libc::{c_char, c_int, c_ulong};
use std::{
    thread::{spawn, JoinHandle},
    u16,
};
use uinput::{
    event::{self, controller::GamePad, Controller},
    Device as UInputDevice,
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
        Self::new()
    }
}

#[cfg(feature = "xtst")]
impl Copy for DisplayPtr {}

#[cfg(feature = "xtst")]
impl DisplayPtr {
    fn new() -> Self {
        let mut null: c_char = 0;
        unsafe {
            Self {
                0: XOpenDisplay(&mut null),
            }
        }
    }
}

impl Peripheral {
    fn new(path: String, suffix: String) -> Result<Self> {
        let evdev_device = EvdevDevice::open(&path)?;
        let (x11connection, _) = RustConnection::connect(None)?;

        Ok(Self {
            path,
            suffix,
            evdev_device,
            uinput_device: None,
            devicetype: DeviceType::Unknown,
            configured: false,
            display: None,
            x11connection,
        })
    }

    fn configure(&mut self) -> Result<()> {
        self.configured = true;
        if self.devicetype == DeviceType::Gamepad {
            self.uinput_device = Some(
                uinput::default()?
                    .name(format!("Splinux Virtual Gamepad Device {}", self.suffix))?
                    .event(event::Controller::All)?
                    .create()?,
            );
        } else if self.devicetype == DeviceType::Mouse {
            #[cfg(feature = "xtst")]
            {
                self.display = Some(DisplayPtr::new());
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

    fn run(&mut self) -> Result<()> {
        if let Err(x) = self.evdev_device.grab() {
            eprintln!("Couldn't grab the input device, continuing anyways.");
            eprintln!("{}: {}", x.kind(), x.to_string());
        }

        loop {
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

        _ => GamePad::Mode,
    }))
}

pub fn client(devices: String) {
    let dev_nums: Vec<&str> = devices.split(",").collect();
    let mut handles: Vec<JoinHandle<()>> = vec![];

    for device_num in dev_nums {
        let path = format!("/dev/input/event{}", device_num);
        let mut perip = Peripheral::new(path, device_num.to_owned()).unwrap();

        let handle = spawn(move || {
            perip.run().unwrap();
        });

        handles.push(handle);
    }

    for handle in handles {
        // if one thread fails, the other should too.
        // this prevents lockups, because disconnecting one device frees the other (perhaps unpluggable) device
        handle.join().unwrap();
    }
}
