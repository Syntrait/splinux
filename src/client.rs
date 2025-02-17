use enigo::{Direction, Enigo, Keyboard, Mouse, Settings};
use evdev::{Device, KeyCode};
use std::{
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

fn evdev_to_char(key: KeyCode) -> Option<char> {
    match key {
        // TODO: Gamepad?
        KeyCode::KEY_A => return Some('a'),
        KeyCode::KEY_B => return Some('b'),
        KeyCode::KEY_C => return Some('c'),
        KeyCode::KEY_D => return Some('d'),
        KeyCode::KEY_E => return Some('e'),
        KeyCode::KEY_F => return Some('f'),
        KeyCode::KEY_G => return Some('g'),
        KeyCode::KEY_H => return Some('h'),
        KeyCode::KEY_I => return Some('i'),
        KeyCode::KEY_J => return Some('j'),
        KeyCode::KEY_K => return Some('k'),
        KeyCode::KEY_L => return Some('l'),
        KeyCode::KEY_M => return Some('m'),
        KeyCode::KEY_N => return Some('n'),
        KeyCode::KEY_O => return Some('o'),
        KeyCode::KEY_P => return Some('p'),
        KeyCode::KEY_Q => return Some('q'),
        KeyCode::KEY_R => return Some('r'),
        KeyCode::KEY_S => return Some('s'),
        KeyCode::KEY_T => return Some('t'),
        KeyCode::KEY_U => return Some('u'),
        KeyCode::KEY_V => return Some('v'),
        KeyCode::KEY_W => return Some('w'),
        KeyCode::KEY_X => return Some('x'),
        KeyCode::KEY_Y => return Some('y'),
        KeyCode::KEY_Z => return Some('z'),

        KeyCode::KEY_0 => return Some('0'),
        KeyCode::KEY_1 => return Some('1'),
        KeyCode::KEY_2 => return Some('2'),
        KeyCode::KEY_3 => return Some('3'),
        KeyCode::KEY_4 => return Some('4'),
        KeyCode::KEY_5 => return Some('5'),
        KeyCode::KEY_6 => return Some('6'),
        KeyCode::KEY_7 => return Some('7'),
        KeyCode::KEY_8 => return Some('8'),
        KeyCode::KEY_9 => return Some('9'),

        KeyCode::KEY_SLASH => return Some('/'),
        KeyCode::KEY_APOSTROPHE => return Some('\"'),
        KeyCode::KEY_SEMICOLON => return Some(';'),
        KeyCode::KEY_MINUS => return Some('-'),

        KeyCode::KEY_SPACE => return Some(' '),

        _ => {}
    }

    None
}

fn evdev_to_enigo_key(key: KeyCode) -> Option<enigo::Key> {
    match key {
        KeyCode::KEY_LEFTALT => {
            return Some(enigo::Key::Alt);
        }
        KeyCode::KEY_RIGHTALT => {
            return Some(enigo::Key::Alt);
        }
        KeyCode::KEY_LEFTMETA => {
            return Some(enigo::Key::Meta);
        }
        KeyCode::KEY_RIGHTMETA => {
            return Some(enigo::Key::Meta);
        }
        KeyCode::KEY_LEFTSHIFT => {
            return Some(enigo::Key::LShift);
        }
        KeyCode::KEY_RIGHTSHIFT => {
            return Some(enigo::Key::RShift);
        }
        KeyCode::KEY_CAPSLOCK => {
            return Some(enigo::Key::CapsLock);
        }
        KeyCode::KEY_LEFTCTRL => {
            return Some(enigo::Key::LControl);
        }
        KeyCode::KEY_RIGHTCTRL => {
            return Some(enigo::Key::RControl);
        }
        KeyCode::KEY_BACKSPACE => {
            return Some(enigo::Key::Backspace);
        }
        KeyCode::KEY_DELETE => {
            return Some(enigo::Key::Delete);
        }
        KeyCode::KEY_UP => {
            return Some(enigo::Key::UpArrow);
        }
        KeyCode::KEY_DOWN => {
            return Some(enigo::Key::DownArrow);
        }
        KeyCode::KEY_LEFT => {
            return Some(enigo::Key::LeftArrow);
        }
        KeyCode::KEY_RIGHT => {
            return Some(enigo::Key::RightArrow);
        }
        KeyCode::KEY_HOME => {
            return Some(enigo::Key::Home);
        }
        KeyCode::KEY_END => {
            return Some(enigo::Key::End);
        }
        KeyCode::KEY_ESC => {
            return Some(enigo::Key::Escape);
        }
        KeyCode::KEY_INSERT => {
            return Some(enigo::Key::Insert);
        }
        KeyCode::KEY_NUMLOCK => {
            return Some(enigo::Key::Numlock);
        }
        KeyCode::KEY_PAGEUP => {
            return Some(enigo::Key::PageUp);
        }
        KeyCode::KEY_PAGEDOWN => {
            return Some(enigo::Key::PageDown);
        }
        KeyCode::KEY_PRINT => {
            return Some(enigo::Key::SysReq);
        }

        KeyCode::KEY_F1 => {
            return Some(enigo::Key::F1);
        }
        KeyCode::KEY_F2 => {
            return Some(enigo::Key::F2);
        }
        KeyCode::KEY_F3 => {
            return Some(enigo::Key::F3);
        }
        KeyCode::KEY_F4 => {
            return Some(enigo::Key::F4);
        }
        KeyCode::KEY_F5 => {
            return Some(enigo::Key::F5);
        }
        KeyCode::KEY_F6 => {
            return Some(enigo::Key::F6);
        }
        KeyCode::KEY_F7 => {
            return Some(enigo::Key::F7);
        }
        KeyCode::KEY_F8 => {
            return Some(enigo::Key::F8);
        }
        KeyCode::KEY_F9 => {
            return Some(enigo::Key::F9);
        }
        KeyCode::KEY_F10 => {
            return Some(enigo::Key::F10);
        }
        KeyCode::KEY_F11 => {
            return Some(enigo::Key::F11);
        }
        KeyCode::KEY_F12 => {
            return Some(enigo::Key::F12);
        }

        _ => {}
    }

    None
}

pub fn client(devices: String, display: String, mita: bool) {
    let mut settings = Settings::default();
    if display.contains(":") {
        settings.x11_display = Some(":11".to_owned());
        settings.wayland_display = None;
    } else if display.contains("wayland-") {
        settings.x11_display = None;
        settings.wayland_display = Some(display.to_owned());
    }

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

            let mitav: Vec<char> = "Praying for you 🕯️ O Great Mita 💝\n".chars().collect();
            let mut miterator = mitav.into_iter().cycle();

            loop {
                for e in device.fetch_events().unwrap() {
                    // TODO: implement mouse move, key presses, mouse scroll, and gamepad
                    match e.destructure() {
                        evdev::EventSummary::Key(_, code, value) => {
                            if value == 1 {
                                if mita {
                                    enigo
                                        .key(
                                            enigo::Key::Unicode(miterator.next().unwrap()),
                                            Direction::Click,
                                        )
                                        .unwrap();
                                } else {
                                    if let Some(char) = evdev_to_char(code) {
                                        enigo
                                            .key(enigo::Key::Unicode(char), Direction::Press)
                                            .unwrap();
                                    } else {
                                    }
                                }
                            } else if value == 0 {
                                if let Some(char) = evdev_to_char(code) {
                                    enigo
                                        .key(enigo::Key::Unicode(char), Direction::Release)
                                        .unwrap();
                                }
                            }

                            println!("space is {:#?}", enigo::Key::Space);
                            println!("Keycode meaning according to evdev: {:#?}", code);
                            println!("Keycode according to evdev: {}", code.0);
                            println!(
                                "Keycode meaning according to char: {}",
                                char::from_u32(code.0 as u32).unwrap()
                            );
                            println!(
                                "Keycode according to keycode_to_char: {}",
                                evdev_to_char(code)
                            );
                        }
                        evdev::EventSummary::RelativeAxis(_, code, value) => {
                            // TODO: mouse implementation
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
