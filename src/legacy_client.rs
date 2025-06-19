use crate::types::{StateCommand, REL_WHEEL, REL_X, REL_Y};
use eframe::glow::NONE;
use evdev::{Device, KeyCode, RelativeAxisCode};
use flume::{unbounded, Selector};
use std::thread::{spawn, JoinHandle};
use x11rb::{
    connection::Connection,
    protocol::{
        xfixes::ConnectionExt as xfixescext,
        xinput::{list_input_devices, ConnectionExt, DeviceInfo, DeviceUse},
        xproto::{ConnectionExt as xproto_ext, MOTION_NOTIFY_EVENT},
        xtest::ConnectionExt as xtest_ext,
    },
    rust_connection::RustConnection,
};

pub fn client(devices: String) {
    let dev_nums: Vec<&str> = devices.split(",").collect();
    let mut handles: Vec<JoinHandle<()>> = vec![];

    let (tx, rx) = unbounded::<StateCommand>();

    let state_handle = spawn(move || {
        let mut fpsmode = false;
        loop {
            match rx.recv() {
                Ok(x) => match x {
                    StateCommand::ToggleFPSMode => {
                        fpsmode = !fpsmode;
                    }
                    StateCommand::GetFPSMode(callback) => {
                        callback.send(fpsmode).unwrap();
                    }
                },
                Err(_) => break,
            }
        }
    });

    for device_num in dev_nums {
        let mut device = Device::open(format!("/dev/input/event{}", device_num)).unwrap();
        let tx = tx.clone();

        let handle = spawn(move || {
            let (conn, screen_num) = RustConnection::connect(None).unwrap();
            let screen = &conn.setup().roots[screen_num];
            let window = screen.root;

            if let Err(x) = device.grab() {
                println!("Couldn't grab the input device, continuing anyways.");
                println!("{}: {}", x.kind(), x.to_string());
            }

            loop {
                for e in device.fetch_events().unwrap() {
                    match e.destructure() {
                        evdev::EventSummary::Key(_, keycode, x) => {
                            match keycode.0 {
                                mbutton @ 272..=274 => {
                                    conn.xtest_fake_input(
                                        if x == 1 {
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
                                }
                                y => {
                                    if x == 2 {
                                        // constant hold events, we only care about start and stop
                                        continue;
                                    }
                                    conn.xtest_fake_input(
                                        if x == 1 {
                                            x11rb::protocol::xproto::KEY_PRESS_EVENT
                                        } else {
                                            x11rb::protocol::xproto::KEY_RELEASE_EVENT
                                        },
                                        // if you add +8 to libinput key id, you get x11 key id for the corresponding key
                                        y as u8 + 8,
                                        0,
                                        NONE,
                                        0,
                                        0,
                                        0,
                                    )
                                    .unwrap();
                                }
                            }
                        }
                        evdev::EventSummary::RelativeAxis(_, code, value) => match code {
                            RelativeAxisCode::REL_X => {
                                let (mstx, msrx) = flume::bounded::<bool>(0);
                                tx.send(StateCommand::GetFPSMode(mstx)).unwrap();
                                let fpslock = msrx.recv().unwrap();

                                if fpslock {
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
                                } else {
                                    let curpos =
                                        conn.query_pointer(window).unwrap().reply().unwrap();
                                    let (x, y) = (curpos.root_x, curpos.root_y);
                                    conn.warp_pointer(NONE, NONE, x, y, 0, 0, value as i16, 0)
                                        .unwrap();
                                }
                            }
                            RelativeAxisCode::REL_Y => {
                                /*

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
                                */
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
