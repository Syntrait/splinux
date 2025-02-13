use enigo::{Enigo, Keyboard, Mouse, Settings};
use std::{thread::sleep, time::Duration};

pub fn client(devices: String, display: String) {
    let mut settings = Settings::default();
    if display.contains(":") {
        settings.x11_display = Some(":11".to_owned());
        settings.wayland_display = None;
    } else {
        settings.x11_display = None;
        settings.wayland_display = Some("wayland-2".to_owned());
    }

    let mut enigo = Enigo::new(&settings).unwrap();

    sleep(Duration::from_secs(5));

    enigo.move_mouse(10, 10, enigo::Coordinate::Rel).unwrap();
    enigo
        .key(enigo::Key::Space, enigo::Direction::Click)
        .unwrap();
}
