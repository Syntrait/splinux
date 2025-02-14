use enigo::{Enigo, Keyboard, Mouse, Settings};
use evdev::{Device, KeyCode};
use std::thread::{sleep, spawn, JoinHandle};

pub fn client(devices: String, display: String) {
    let mut settings = Settings::default();
    if display.contains(":") {
        settings.x11_display = Some(":11".to_owned());
        settings.wayland_display = None;
    } else if display.contains("wayland-") {
        settings.x11_display = None;
        settings.wayland_display = Some(display.to_owned());
    }

    /*
    enigo.move_mouse(10, 10, enigo::Coordinate::Rel).unwrap();
    enigo
        .key(enigo::Key::Space, enigo::Direction::Click)
        .unwrap();
        */

    let dev_nums: Vec<&str> = devices.split(",").collect();

    let mut handles: Vec<JoinHandle<()>> = vec![];

    for device_num in dev_nums {
        let mut device = Device::open(format!("/dev/input/event{}", device_num)).unwrap();
        let settings = settings.clone();
        let handle: JoinHandle<()> = spawn(move || {
            let mut enigo = Enigo::new(&settings).unwrap();
            if let Err(x) = device.grab() {
                println!("Couldn't grab the input device, continuing anyways.");
                println!("{}: {}", x.kind(), x.to_string());
            }
            loop {
                for e in device.fetch_events().unwrap() {
                    // TODO: implement mouse move, key presses, mouse scroll
                    match e.destructure() {
                        evdev::EventSummary::Key(_, code, value) => {
                            enigo
                                .key(enigo::Key::Space, enigo::Direction::Click)
                                .unwrap();
                            println!("code: {:#?}, value: {:#?}", code, value);

                            println!("space is {}", enigo::Key::Space.try_into().unwrap());
                            println!("{}", code.0);
                        }
                        evdev::EventSummary::RelativeAxis(_, code, value) => {
                            println!("code: {:#?}, value: {:#?}", code, value);
                        }
                        _ => {}
                    }
                }
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }
}
