use std::{
    env::var,
    path::PathBuf,
    str::{FromStr, from_utf8},
};

use anyhow::{Result, anyhow};

pub struct LaunchPreferences {
    AppID: u32,
    IsNative: bool,
    ExecutablePath: PathBuf,
    InstallPath: PathBuf,
    ProtonPath: Option<PathBuf>,
}

enum VdfReadSteps {
    AppID,
    ProtonVer,
    Ignore(usize),
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

fn read_config_vdf() -> Result<Vec<(u32, String)>> {
    let home = var("HOME")?;
    let config_vdf = PathBuf::from(format!("{}/.steam/steam/config/config.vdf", home));

    if !config_vdf.exists() {
        return Err(anyhow!(
            "config.vdf couldn't be found in ~/.steam/steam/config"
        ))?;
    }

    let data = std::fs::read(config_vdf)?;
    let data = from_utf8(&data)?;
    let data = data
        .split("\"CompatToolMapping\"")
        .nth(1)
        .ok_or(anyhow!("Key 'CompatToolMapping' is missing in config.vdf"))?
        .split("\"depots\"")
        .nth(0)
        .ok_or(anyhow!("Key 'depots' is missing in config.vdf"))?;

    let data = data.split("{").skip(1);

    let mut vdf_vec: Vec<(u32, String)> = vec![];

    let mut step = VdfReadSteps::AppID;

    let mut appid = 0;
    let mut protonver: Box<String> = Box::new(String::new());
    let mut debounce = false;

    for item in data {
        let lines: Vec<&str> = item.split("\n").collect();
        let linecount = lines.len();
        if linecount == 3 {
            appid = 0;
            step = VdfReadSteps::ProtonVer;
            continue;
        }
        for line in lines {
            let linetrim = line.trim();
            if linetrim.is_empty() {
                continue;
            }
            match step {
                VdfReadSteps::AppID => {
                    let temp_appid: Result<u32, _> = linetrim.replace("\"", "").parse();

                    if let Err(err) = temp_appid {
                        if debounce {
                            // parsing is done
                            break;
                        } else {
                            return Err(err)?;
                        }
                    } else {
                        appid = temp_appid?;
                    }

                    step = VdfReadSteps::ProtonVer;
                }
                VdfReadSteps::ProtonVer => {
                    let mut splits: Vec<&str> = linetrim.split("\"").skip(3).collect();
                    splits.pop();
                    protonver = Box::new(splits.join("\""));

                    step = VdfReadSteps::Ignore(3);
                }
                VdfReadSteps::Ignore(count) => {
                    if count == 1 {
                        vdf_vec.push((appid, *protonver.clone()));
                        debounce = true;
                        step = VdfReadSteps::AppID;
                    } else {
                        step = VdfReadSteps::Ignore(count - 1);
                    }
                }
            }
        }
    }

    println!("vdf_vec: {:#?}", vdf_vec);

    Ok(vdf_vec)
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

    read_config_vdf()?;

    // TODO: Parsing

    Ok(LaunchPreferences {
        AppID: appid,
        IsNative: true,
        InstallPath: installpath,
        ExecutablePath: PathBuf::from_str("hello").unwrap(),
        ProtonPath: PathBuf::from_str("hello").ok(),
    })
}
