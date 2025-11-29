use std::{
    env::var,
    path::PathBuf,
    str::{FromStr, from_utf8},
};

use anyhow::{Ok, Result, anyhow};

pub struct LaunchPreferences {
    AppID: u32,
    IsNative: bool,
    ExecutablePath: PathBuf,
    InstallPath: PathBuf,
    ProtonPath: Option<PathBuf>,
}

pub fn find_libraries() -> Result<Vec<PathBuf>> {
    let mut libraries = vec![];

    let home = var("HOME")?;
    let steamapps = PathBuf::from(home).join(".steam/steam/steamapps");

    if steamapps.exists() {
        println!("steamapps is normal!!");
        libraries.push(steamapps);
    }

    if libraries.is_empty() {
        Err(anyhow!("Steam library location not found"))?
    }
    return Ok(libraries);
}

pub fn get_launch_preferences(appid: u32) -> Result<LaunchPreferences> {
    let list_of_libraries = find_libraries()?;

    let mut library = PathBuf::new();

    for item_library in list_of_libraries {
        if item_library.exists()
            && item_library
                .join(format!("appmanifest_{}.acf", appid))
                .exists()
        {
            library = item_library;
            break;
        }
    }
    if !library.exists() {
        Err(anyhow!("The game couldn't be found in any libraries"))?
    }

    let acf_config = library.join(format!("appmanifest_{}.acf", appid));
    let acf_config = std::fs::read(acf_config)?;
    let acf_config = from_utf8(&acf_config)?;

    let mut installdir = String::new();

    for line in acf_config.split("\n") {
        if line.contains("\"installdir\"") {
            let mut parsvec: Vec<&str> = line.split("\"").skip(3).collect();

            parsvec
                .pop()
                .ok_or(anyhow!("appmanifest parsing failed on pop"))?;

            let pars = parsvec.join("\"");

            installdir = pars.to_owned();
            break;
        }
    }
    let installpath = library.join("common").join(installdir);
    if !installpath.exists() {
        return Err(anyhow!("Game directory is missing"))?;
    }

    // TODO: Parsing

    Ok(LaunchPreferences {
        AppID: appid,
        IsNative: true,
        InstallPath: installpath,
        ExecutablePath: PathBuf::from_str("hello").unwrap(),
        ProtonPath: PathBuf::from_str("hello").ok(),
    })
}
