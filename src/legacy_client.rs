use crate::types::{
    DeviceType, BTN_EAST, BTN_NORTH, BTN_SOUTH, BTN_THUMBL, BTN_THUMBR, BTN_TL, BTN_TL2, BTN_TR,
    BTN_TR2, BTN_WEST,
};
use eframe::glow::NONE;
use evdev::{Device, RelativeAxisCode};
#[cfg(feature = "xtst")]
use libc::{c_char, c_int, c_ulong};
use std::{
    panic,
    thread::{spawn, JoinHandle},
    u16,
};
use uinput::event::{self, controller::GamePad, Controller};
use x11rb::{
    connection::Connection, protocol::xtest::ConnectionExt as xtest_ext,
    rust_connection::RustConnection,
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
        let mut device = Device::open(format!("/dev/input/event{}", device_num)).unwrap();

        let handle = spawn(move || {
            let (conn, _) = RustConnection::connect(None).unwrap();

            let mut devtype = DeviceType::Unknown;
            let mut configured = false;
            let mut uinputdevice: Option<uinput::Device> = None;
            let mut display: Option<*mut Display> = None;

            if let Err(x) = device.grab() {
                println!("Couldn't grab the input device, continuing anyways.");
                println!("{}: {}", x.kind(), x.to_string());
            }

            loop {
                if !configured && devtype != DeviceType::Unknown {
                    configured = true;
                    if devtype == DeviceType::Gamepad {
                        uinputdevice = Some(
                            uinput::default()
                                .unwrap()
                                .name("Virtual Gamepad")
                                .unwrap()
                                .event(event::Controller::All)
                                .unwrap()
                                .create()
                                .unwrap(),
                        );
                    } else if devtype == DeviceType::Mouse {
                        #[cfg(feature = "xtst")]
                        {
                            unsafe {
                                let mut null: c_char = 0;
                                display = Some(XOpenDisplay(&mut null));
                                if let Some(display) = display.as_mut() {
                                    if display.is_null() {
                                        panic!("Failed to open DISPLAY");
                                    }
                                }
                            }
                        }
                    }
                }
                // TODO: structify this mess
                for e in device.fetch_events().unwrap() {
                    match e.destructure() {
                        evdev::EventSummary::Key(_, keycode, keystate) => {
                            match keycode.0 {
                                mbutton @ 272..=274 => {
                                    devtype = DeviceType::Mouse;
                                    conn.xtest_fake_input(
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
                                        NONE,
                                        0,
                                        0,
                                        0,
                                    )
                                    .unwrap();

                                    conn.flush().unwrap();
                                }
                                gamepadbutton @ 304..=318 => {
                                    devtype = DeviceType::Gamepad;
                                    if !configured {
                                        continue;
                                    };
                                    if let Some(dev) = uinputdevice.as_mut() {
                                        dev.send(
                                            libinput_key_to_uinput_event(gamepadbutton),
                                            keystate,
                                        )
                                        .unwrap();

                                        dev.synchronize().unwrap();
                                    }
                                }
                                keyid => {
                                    if keystate == 2 {
                                        // constant hold events, we only care about start and stop
                                        continue;
                                    }
                                    devtype = DeviceType::Keyboard;
                                    conn.xtest_fake_input(
                                        if keystate == 1 {
                                            x11rb::protocol::xproto::KEY_PRESS_EVENT
                                        } else {
                                            x11rb::protocol::xproto::KEY_RELEASE_EVENT
                                        },
                                        // if you add +8 to libinput key id, you get x11 key id for the corresponding key
                                        keyid as u8 + 8,
                                        0,
                                        NONE,
                                        0,
                                        0,
                                        0,
                                    )
                                    .unwrap();

                                    conn.flush().unwrap();
                                }
                            }
                        }
                        evdev::EventSummary::RelativeAxis(_, code, value) => match code {
                            RelativeAxisCode::REL_X => {
                                devtype = DeviceType::Mouse;
                                if !configured {
                                    continue;
                                }
                                #[cfg(feature = "xtst")]
                                {
                                    if let Some(display) = display.as_mut() {
                                        unsafe {
                                            let display = display.to_owned();
                                            let success =
                                                XTestFakeRelativeMotionEvent(display, value, 0, 0);
                                            if success == 0 {
                                                panic!("relative movement failed")
                                            }
                                            XFlush(display);
                                        }
                                    }
                                }
                                #[cfg(not(feature = "xtst"))]
                                {
                                    // absolute cursor only
                                    conn.warp_pointer(NONE, window, 0, 0, 0, 0, value as i16, 0)
                                        .unwrap();

                                    conn.flush.unwrap();
                                }
                            }
                            RelativeAxisCode::REL_Y => {
                                devtype = DeviceType::Mouse;
                                if !configured {
                                    continue;
                                }
                                #[cfg(feature = "xtst")]
                                {
                                    if let Some(display) = display.as_mut() {
                                        unsafe {
                                            let display = display.to_owned();
                                            let success =
                                                XTestFakeRelativeMotionEvent(display, 0, value, 0);
                                            if success == 0 {
                                                panic!("relative movement failed")
                                            }
                                            XFlush(display);
                                        }
                                    }
                                }

                                #[cfg(not(feature = "xtst"))]
                                {
                                    // TODO: add createcursor
                                    // absolute cursor only
                                    conn.warp_pointer(NONE, window, 0, 0, 0, 0, 0, value as i16)
                                        .unwrap();

                                    conn.flush.unwrap();
                                }
                            }
                            RelativeAxisCode::REL_WHEEL => {
                                devtype = DeviceType::Mouse;
                                conn.xtest_fake_input(
                                    x11rb::protocol::xproto::BUTTON_PRESS_EVENT,
                                    if value == 1 { 4 } else { 5 },
                                    0,
                                    NONE,
                                    0,
                                    0,
                                    0,
                                )
                                .unwrap();

                                conn.xtest_fake_input(
                                    x11rb::protocol::xproto::BUTTON_RELEASE_EVENT,
                                    if value == 1 { 4 } else { 5 },
                                    0,
                                    NONE,
                                    0,
                                    0,
                                    0,
                                )
                                .unwrap();

                                conn.flush().unwrap();
                            }

                            _ => {}
                        },

                        _ => {}
                    }
                }
            }
        });

        handles.push(handle);
    }
    for handle in handles {
        // if one thread fails, the other should too.
        // this prevents lockups, because disconnecting one device frees the other (perhaps unpluggable) device
        handle.join().unwrap();
    }
}
