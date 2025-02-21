use crate::types::{REL_WHEEL, REL_X, REL_Y};
use eframe::glow::NONE;
use evdev::{Device, KeyCode, RelativeAxisCode};
use std::{
    process,
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};
use x11rb::{
    connection::Connection,
    protocol::{xproto::ConnectionExt as xproto_ext, xtest::ConnectionExt as xtest_ext},
    rust_connection::RustConnection,
};

pub fn client(devices: String) {
    let dev_nums: Vec<&str> = devices.split(",").collect();
    let mut handles: Vec<JoinHandle<()>> = vec![];

    for device_num in dev_nums {
        let mut device = Device::open(format!("/dev/input/event{}", device_num)).unwrap();

        let handle = spawn(move || {
            let (conn, screen_num) = RustConnection::connect(None).unwrap_or_else(|x| {
                eprintln!("{}", x);
                process::exit(1);
            });
            let screen = &conn.setup().roots[screen_num];
            let window = screen.root;

            if let Err(x) = device.grab() {
                println!("Couldn't grab the input device, continuing anyways.");
                println!("{}: {}", x.kind(), x.to_string());
            }
            loop {
                for e in device.fetch_events().unwrap() {
                    match e.destructure() {
                        evdev::EventSummary::Key(keyevent, keycode, x) => {
                            println!(
                                "keyevent: {:#?}, keycode: {:#?}, x: {:#?}",
                                keyevent, keycode, x
                            );
                        }
                        evdev::EventSummary::RelativeAxis(_, code, value) => match code {
                            RelativeAxisCode::REL_X => {
                                let curpos = conn.query_pointer(window).unwrap().reply().unwrap();

                                conn.warp_pointer(NONE, NONE, 0, 0, 0, 0, value as i16, 0)
                                    .unwrap();

                                let curpos2 = conn.query_pointer(window).unwrap().reply().unwrap();

                                if curpos.root_x == curpos2.root_x {
                                    conn.xtest_fake_input(
                                        x11rb::protocol::xproto::MOTION_NOTIFY_EVENT,
                                        0,
                                        0,
                                        NONE,
                                        value as i16,
                                        0,
                                        0,
                                    )
                                    .unwrap();
                                }
                            }
                            RelativeAxisCode::REL_Y => {
                                let curpos = conn.query_pointer(window).unwrap().reply().unwrap();

                                conn.warp_pointer(NONE, NONE, 0, 0, 0, 0, 0, value as i16)
                                    .unwrap();

                                let curpos2 = conn.query_pointer(window).unwrap().reply().unwrap();

                                if curpos.root_y == curpos2.root_y {
                                    conn.xtest_fake_input(
                                        x11rb::protocol::xproto::MOTION_NOTIFY_EVENT,
                                        0,
                                        0,
                                        NONE,
                                        0,
                                        value as i16,
                                        0,
                                    )
                                    .unwrap();
                                }
                            }
                            RelativeAxisCode::REL_WHEEL => {
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
                            }

                            _ => {}
                        },

                        _ => {}
                    }

                    conn.flush().unwrap();
                }
            }
        });

        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }
}
