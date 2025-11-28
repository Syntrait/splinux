use std::env::var;
use std::fs;
use std::path::PathBuf;

pub fn init_saves() {
    let maindir = construct_main_dir();

    fs::DirBuilder::new()
        .recursive(true)
        .create(maindir.join("saves"))
        .unwrap();

    fs::DirBuilder::new()
        .recursive(true)
        .create(maindir.join("presets"))
        .unwrap();

    fs::DirBuilder::new()
        .recursive(true)
        .create(maindir.join("baseprefix"))
        .unwrap();
}

pub fn construct_main_dir() -> PathBuf {
    let home = var("HOME").unwrap();

    format!("{}/.local/share/splinux", home).into()
}
