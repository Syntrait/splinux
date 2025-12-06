use std::{
    env::var,
    fs::{DirEntry, read_dir},
    path::PathBuf,
    str::{FromStr, from_utf8},
};

use anyhow::{Ok, Result, anyhow};
use memchr::memchr_iter;

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

// This is the prefix for executable name
const MAGIC_BYTES: [u8; 4] = [0xC6, 0x01, 0x00, 0x00];

// This is the suffix (linux)
const MAGIC_BYTES2: [u8; 6] = [0x6C, 0x69, 0x6E, 0x75, 0x78, 0x00];

pub fn get_executable_path(appid: u32, isnative: bool, library: &PathBuf) -> Result<PathBuf> {
    // involves reading appinfo.vdf
    let home = var("HOME")?;
    let appinfo_vdf = std::fs::read(format!("{}/.steam/steam/appcache/appinfo.vdf", home))?;
    let appid_bytes: [u8; 4] = appid.to_le_bytes();

    let appid_search = memchr_iter(appid_bytes[0], &appinfo_vdf)
        .filter(|&x| appinfo_vdf[x + 1..].starts_with(&appid_bytes[1..]));

    let appid_indexes: Vec<usize> = appid_search.collect();

    assert!(appid_indexes.len() >= 3);

    // appid search is done

    let secondmatch_index = appid_indexes[1];

    let name = &appinfo_vdf[secondmatch_index + 14..secondmatch_index + 14 + 30];

    let namepos = name.windows(2).position(|x| x == [0x00, 0x01]).unwrap();

    let name = &name[..namepos];

    let name = String::from_utf8_lossy(name).into_owned();

    println!("name: {}", name);

    let appinfo_vdf = &appinfo_vdf[secondmatch_index..];

    let search = memchr_iter(MAGIC_BYTES[0], &appinfo_vdf)
        .filter(|&x| appinfo_vdf[x + 1..].starts_with(&MAGIC_BYTES[1..]))
        .next()
        .unwrap();

    match isnative {
        true => {
            // if it is a native game

            let appinfo_vdf = &appinfo_vdf[search + MAGIC_BYTES.len()..];

            let index = memchr_iter(MAGIC_BYTES2[0], appinfo_vdf)
                .filter(|&x| appinfo_vdf[x + 1..].starts_with(&MAGIC_BYTES2[1..]))
                .next()
                .unwrap();

            let range_slice = &appinfo_vdf[index - 80..index];

            let mut interest_iter = range_slice
                .windows(MAGIC_BYTES.len())
                .enumerate()
                .filter(|(_, b)| b.starts_with(&MAGIC_BYTES));

            let mut interest = interest_iter.next().unwrap();

            if let Some(new_interest) = interest_iter.next() {
                interest = new_interest;
            }

            let interest = interest.0;

            let range_slice = &range_slice[interest + 4..];

            let range_slice_index = range_slice
                .windows(2)
                .position(|x| x == [0x00, 0x01])
                .unwrap();

            let range_slice = &range_slice[..range_slice_index];

            let stringified = String::from_utf8_lossy(range_slice).into_owned();
            println!("stringified: {:#?}", stringified);

            let executable_path = library.join("common").join(name).join(stringified);

            if executable_path.exists() {
                return Ok(executable_path);
            } else {
                return Err(anyhow!("executable_path doesn't exist"));
            }
        }
        false => {
            // if not a native game

            let executable_name_endindex = appinfo_vdf[search + MAGIC_BYTES.len()..]
                .windows(2)
                .position(|x| x == [0x00, 0x01])
                .unwrap();

            let executable_name = &appinfo_vdf
                [search + MAGIC_BYTES.len()..search + MAGIC_BYTES.len() + executable_name_endindex];

            let executable_name = String::from_utf8_lossy(executable_name);
            let executable_name = executable_name.into_owned();

            println!("executable_name: {}", executable_name);

            let executable_path = library.join("common").join(name).join(executable_name);

            if executable_path.exists() {
                return Ok(executable_path);
            } else {
                return Err(anyhow!("executable_path doesn't exist"));
            }
        }
    }
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

pub fn list_protons() -> Result<Vec<PathBuf>> {
    let home = var("HOME")?;
    let compat_tools = PathBuf::from(home).join(".steam/steam/compatibilitytools.d");
    let mut protons: Vec<PathBuf> = vec![];

    if compat_tools.exists() {
        let customprotons = read_dir(compat_tools)?;

        let customprotons = customprotons
            .filter_map(|dir| dir.ok())
            .filter(|entry| entry.file_type().is_ok_and(|abc| abc.is_dir()))
            .map(|direntry| direntry.path());

        protons.extend(customprotons);
    }

    let list_of_libraries = find_libraries()?;

    for library in list_of_libraries {
        let common = library.join("common");

        if common.exists() {
            let apps = read_dir(common)?;
            let commonapps = apps
                .filter_map(|app| app.ok())
                .filter(|app| {
                    app.file_name()
                        .into_string()
                        .is_ok_and(|filename| filename.starts_with("Proton "))
                })
                .filter(|app| app.file_type().is_ok_and(|ftype| ftype.is_dir()))
                .filter(|app| app.path().join("proton").exists())
                .map(|app| app.path());

            protons.extend(commonapps);
        }
    }

    Ok(protons)
}

impl LaunchPreferences {
    pub fn new(appid: u32, protonpath: Option<PathBuf>) -> Result<LaunchPreferences> {
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

        let proton_preference = read_config_vdf()?;
        // TODO: check if the game is native

        Ok(LaunchPreferences {
            AppID: appid,
            IsNative: true,
            InstallPath: installpath,
            ExecutablePath: PathBuf::from_str("hello").unwrap(),
            ProtonPath: protonpath,
        })
    }
}
