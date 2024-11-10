use crate::types::{REL_WHEEL, REL_X, REL_Y};
use eframe::glow::NONE;
use evdev::Device;
use std::{
    process,
    thread::{sleep, spawn},
    time::Duration,
};
use x11rb::{
    connection::Connection,
    protocol::{xproto::ConnectionExt as xproto_ext, xtest::ConnectionExt as xtest_ext},
    rust_connection::RustConnection,
};

pub fn client(devices: String) {
    let dev_nums: Vec<&str> = devices.split(",").collect();

    for device_num in dev_nums {
        let mut device = Device::open(format!("/dev/input/event{}", device_num)).unwrap();
        spawn(move || {
            let (conn, screen_num) = RustConnection::connect(None).unwrap_or_else(|x| {
                eprintln!("{}", x);
                process::exit(1);
            });
            let screen = &conn.setup().roots[screen_num];
            let window = screen.root;

            device.grab().unwrap_or_else(|err| {
                eprintln!("{:#?}", err);
                process::exit(1);
            });
            loop {
                for e in device.fetch_events().unwrap() {
                    match e.kind() {
                        evdev::InputEventKind::Key(x) => match x.0 {
                            val @ 272..=274 => {
                                conn.xtest_fake_input(
                                    if e.value() == 1 {
                                        x11rb::protocol::xproto::BUTTON_PRESS_EVENT
                                    } else {
                                        x11rb::protocol::xproto::BUTTON_RELEASE_EVENT
                                    },
                                    match val {
                                        272 => 1,
                                        273 => 3,
                                        274 => 2,
                                        _ => 0,
                                    },
                                    0,
                                    NONE,
                                    0,
                                    0,
                                    0,
                                )
                                .unwrap();
                            }
                            x => {
                                if e.value() == 2 {
                                    continue;
                                }
                                conn.xtest_fake_input(
                                    if e.value() == 1 {
                                        x11rb::protocol::xproto::KEY_PRESS_EVENT
                                    } else {
                                        x11rb::protocol::xproto::KEY_RELEASE_EVENT
                                    },
                                    x as u8 + 8,
                                    0,
                                    NONE,
                                    0,
                                    0,
                                    0,
                                )
                                .unwrap();
                            }
                        },
                        evdev::InputEventKind::RelAxis(_) => match e.code() {
                            REL_X => {
                                let curpos = conn.query_pointer(window).unwrap().reply().unwrap();

                                if curpos.root_x as u16 * 2 == screen.width_in_pixels
                                    && curpos.root_y as u16 * 2 == screen.height_in_pixels
                                {
                                    conn.warp_pointer(NONE, NONE, 0, 0, 0, 0, e.value() as i16, 0)
                                        .unwrap();

                                    let curpos2 =
                                        conn.query_pointer(window).unwrap().reply().unwrap();

                                    if curpos2.root_x as u16 * 2 == screen.width_in_pixels {
                                        conn.xtest_fake_input(
                                            x11rb::protocol::xproto::MOTION_NOTIFY_EVENT,
                                            0,
                                            0,
                                            NONE,
                                            e.value() as i16,
                                            0,
                                            0,
                                        )
                                        .unwrap();
                                    }
                                } else {
                                    conn.warp_pointer(NONE, NONE, 0, 0, 0, 0, e.value() as i16, 0)
                                        .unwrap();
                                }
                            }
                            REL_Y => {
                                let curpos = conn.query_pointer(window).unwrap().reply().unwrap();

                                if curpos.root_x as u16 * 2 == screen.width_in_pixels
                                    && curpos.root_y as u16 * 2 == screen.height_in_pixels
                                {
                                    conn.warp_pointer(NONE, NONE, 0, 0, 0, 0, 0, e.value() as i16)
                                        .unwrap();

                                    let curpos2 =
                                        conn.query_pointer(window).unwrap().reply().unwrap();

                                    if curpos2.root_y as u16 * 2 == screen.height_in_pixels {
                                        conn.xtest_fake_input(
                                            x11rb::protocol::xproto::MOTION_NOTIFY_EVENT,
                                            0,
                                            0,
                                            NONE,
                                            0,
                                            e.value() as i16,
                                            0,
                                        )
                                        .unwrap();
                                    }
                                } else {
                                    conn.warp_pointer(NONE, NONE, 0, 0, 0, 0, 0, e.value() as i16)
                                        .unwrap();
                                }
                            }
                            REL_WHEEL => {
                                conn.xtest_fake_input(
                                    x11rb::protocol::xproto::BUTTON_PRESS_EVENT,
                                    if e.value() == 1 { 4 } else { 5 },
                                    0,
                                    NONE,
                                    0,
                                    0,
                                    0,
                                )
                                .unwrap();

                                conn.xtest_fake_input(
                                    x11rb::protocol::xproto::BUTTON_RELEASE_EVENT,
                                    if e.value() == 1 { 4 } else { 5 },
                                    0,
                                    NONE,
                                    0,
                                    0,
                                    0,
                                )
                                .unwrap();
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    conn.flush().unwrap_or_else(|err| {
                        eprintln!("{:#?}", err);
                        process::exit(1);
                    });
                }
            }
        });
    }

    loop {
        sleep(Duration::from_secs(60 * 60 * 24));
    }
}
