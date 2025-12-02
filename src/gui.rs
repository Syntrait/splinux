use std::{env::var, fs::read_to_string, path::PathBuf, thread::spawn};

use crate::{
    native_backend,
    parser::{LaunchPreferences, list_protons},
    saves::{construct_main_dir, init_saves},
    types::{
        Backend, BackendCommand, Client, CommandType, Device, GuiState, Preset, WindowGeometry,
        get_devices,
    },
};
use anyhow::Result;
use eframe::egui::{self, ComboBox, ScrollArea, TextEdit, WidgetText};
use flume::unbounded;
use rfd::FileDialog;

// TODO: do all the launching, save file location spoofing, etc. from the program

struct App {
    clientlist: Vec<Client>,
    guistate: GuiState,
    newpresetname_display: String,
    newclientname_display: String,
    newdevices_display: Vec<GuiDevice>,
    newbackend_display: Backend,
    newgeometry_display: WindowGeometry,
    newcommand_display: CommandType,
    newcommand_display_steamid: String,
    aboutwindow_visible: bool,
    presetlist: Vec<Preset>,
    chosenpreset: Option<Preset>,
    detectedprotons: Vec<PathBuf>,
    newclient_protonver: PathBuf,
}

impl Drop for App {
    fn drop(&mut self) {
        self.clientlist.retain_mut(|x| x.is_alive());
        for client in &mut self.clientlist {
            client.kill();
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            clientlist: vec![],
            guistate: GuiState::MainMenu,
            newpresetname_display: "".to_owned(),
            newclientname_display: "".to_owned(),
            newdevices_display: get_ui_devices(),
            newbackend_display: Backend::Native,
            newgeometry_display: WindowGeometry {
                x: 0,
                y: 0,
                width: 1920,
                height: 540,
            },
            newcommand_display: CommandType::SteamLaunch {
                appid: 0,
                settings: crate::types::SteamSettings::Normal,
            },
            newcommand_display_steamid: "0".to_owned(),
            aboutwindow_visible: false,
            presetlist: vec![],
            chosenpreset: None,
            detectedprotons: vec![],
            newclient_protonver: PathBuf::new(),
        }
    }
}

struct GuiDevice {
    device: Device,
    chosen: bool,
}

fn get_ui_devices() -> Vec<GuiDevice> {
    get_devices()
        .into_iter()
        .map(|dev| GuiDevice {
            device: dev,
            chosen: false,
        })
        .collect()
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _: &mut eframe::Frame) {
        match self.guistate {
            GuiState::MainMenu => self.render_mainmenu(&ctx),
            GuiState::ManagePreset => self.render_managepreset(&ctx),
            GuiState::EditClient => self.render_editclient(&ctx),
            GuiState::EditPreset => self.render_editpreset(&ctx),
            _ => {}
        }
    }
}

impl App {
    fn render_mainmenu(&mut self, ctx: &eframe::egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Splinux");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button("About").clicked() {
                        self.aboutwindow_visible = !self.aboutwindow_visible;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label("Preset name:");
                ui.add(TextEdit::singleline(&mut self.newpresetname_display).desired_width(150.0));
            });
            ui.horizontal(|ui| {
                if ui.button("Create new preset").clicked() {
                    init_saves();

                    if !self.newpresetname_display.is_empty() {
                        let preset = Preset::new(self.newpresetname_display.to_owned(), vec![]);
                        self.presetlist.push(preset);
                    }
                }
                if ui.button("Load a preset").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("YAML config", &["yaml"])
                        .set_directory(construct_main_dir().join("presets"))
                        .pick_file()
                    {
                        if let Ok(content) = read_to_string(file) {
                            let preset: Result<Preset, serde_yaml::Error> =
                                serde_yaml::from_str(&content);

                            if let Ok(preset) = preset {
                                self.presetlist.push(preset);
                            }
                        }
                    }
                }
            });

            if !self.presetlist.is_empty() {
                ui.vertical(|ui| {
                    ScrollArea::both().id_salt("presetlist").show(ui, |ui| {
                        ui.group(|ui| {
                            let mut removeindex: Option<usize> = None;
                            for (index, preset) in self.presetlist.iter().enumerate() {
                                ui.group(|ui| {
                                    ui.label(format!("Preset: {}", preset.name));
                                    if ui.button("Choose").clicked() {
                                        self.chosenpreset = Some(preset.clone());
                                        self.guistate = GuiState::ManagePreset;
                                        removeindex = Some(index);
                                    }
                                    if ui.button("Delete").clicked() {
                                        removeindex = Some(index);
                                    }
                                });
                            }
                            if let Some(index) = removeindex {
                                self.presetlist.remove(index);
                            }
                        });
                    });
                });
            }
        });
        if self.aboutwindow_visible {
            egui::Window::new("About").show(ctx, |ui| {
                ui.label("Splinux");
                ui.label(format!(
                    "Version {}",
                    option_env!("CARGO_PKG_VERSION").unwrap_or("unknown")
                ));
                ui.label("This program comes with absolutely no warranty.");
                ui.hyperlink_to(
                    "See the GNU General Public License, version 3 for details.",
                    "https://www.gnu.org/licenses/gpl-3.0.html",
                );
            });
        }
    }

    fn render_editpreset(&mut self, ctx: &eframe::egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::SidePanel::left("editpresetleftbar")
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        if ui.button("<-").clicked() {
                            self.guistate = GuiState::ManagePreset;
                        }
                        if ui.button("Add Client").clicked() {
                            self.guistate = GuiState::EditClient;
                            self.newdevices_display = get_ui_devices();
                        }
                    });
                });

            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(chosenpreset) = self.chosenpreset.as_mut() {
                    ui.label(format!("Client list for Preset {}:", chosenpreset.name));
                    ScrollArea::both().id_salt("clientlist").show(ui, |ui| {
                        ui.group(|ui| {
                            let mut removeindex: Option<usize> = None;
                            for (index, client) in chosenpreset.clients.iter().enumerate() {
                                ui.group(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(format!("Client: {}", client.name));
                                        ui.label("Devices:");
                                        for dev in client.devices.iter() {
                                            ui.label(format!("- {}", dev.get_name()));
                                        }
                                        if ui.button("Delete").clicked() {
                                            removeindex = Some(index);
                                        }
                                    });
                                });
                            }
                            if let Some(i) = removeindex {
                                chosenpreset.clients.remove(i);
                            }
                        });
                    });
                }
            });
        });
    }

    fn render_managepreset(&mut self, ctx: &eframe::egui::Context) {
        egui::SidePanel::left("managepresetbar")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if ui.button("Overview").clicked() {
                        println!("overview");
                    }
                    if ui.button("Edit Preset").clicked() {
                        self.guistate = GuiState::EditPreset;
                    }
                    if ui.button("Export").clicked() {
                        if let Some(preset) = self.chosenpreset.as_ref() {
                            if let Ok(content) = serde_yaml::to_string(&preset) {
                                if let Some(file) = FileDialog::new()
                                    .add_filter("YAML config", &["yaml"])
                                    .set_directory(construct_main_dir().join("presets"))
                                    .save_file()
                                {
                                    if let Err(err) = std::fs::write(file, content) {
                                        println!("{:#?}", err);
                                    } else {
                                        println!("Successfully exported Preset.");
                                    }
                                }
                            }
                        }
                    }
                    ui.add_space(30.0);
                    if ui.button("Unchoose").clicked() {
                        let preset = self.chosenpreset.take();
                        if let Some(preset) = preset {
                            self.presetlist.push(preset);
                        }
                        self.guistate = GuiState::MainMenu
                    }
                });
            });

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(chosenpreset) = self.chosenpreset.as_ref() {
                ui.label(format!("Preset: {}", chosenpreset.name));
            }
            ui.label("Clients:");
            ScrollArea::both().id_salt("presetoverview").show(ui, |ui| {
                if let Some(chosenpreset) = self.chosenpreset.as_mut() {
                    let mut removeindex: Option<usize> = None;
                    for (index, client) in chosenpreset.clients.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.label(format!("Name: {}", client.name));
                            ui.label("Devices:");
                            for dev in client.devices.iter() {
                                ui.label(format!("- {}", dev.get_name()));
                            }
                            let isclientalive = client.is_alive();
                            if ui
                                .button(if isclientalive { "Stop" } else { "Start" })
                                .clicked()
                            {
                                if isclientalive {
                                    client
                                        .handle
                                        .take()
                                        .unwrap()
                                        .send(BackendCommand::Terminate)
                                        .unwrap();
                                    client.kill();
                                } else {
                                    let (tx, rx) = unbounded::<BackendCommand>();

                                    client.run().unwrap();

                                    client.handle = Some(tx);

                                    let clientdevices = client.devices.clone();
                                    let clientdisplay = client.display.clone();

                                    spawn(move || {
                                        native_backend::backend(clientdevices, clientdisplay, rx);
                                    });
                                }
                            }
                            if ui.button("Delete").clicked() {
                                removeindex = Some(index);
                            }
                        });
                    }
                    if let Some(index) = removeindex {
                        chosenpreset.clients.remove(index);
                    }
                }
            });
        });
    }

    fn render_editclient(&mut self, ctx: &eframe::egui::Context) {
        egui::SidePanel::right("devicelistpanel")
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Devices:");
                    if ui.button("Refresh device list").clicked() {
                        self.newdevices_display = get_ui_devices();
                    }
                });
                if !self.newdevices_display.is_empty() {
                    ui.vertical(|ui| {
                        ScrollArea::both().id_salt("devicelist").show(ui, |ui| {
                            ui.group(|ui| {
                                for device in self.newdevices_display.iter_mut() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(device.device.get_name());
                                            ui.checkbox(&mut device.chosen, "enabled");
                                        });
                                    });
                                }
                            });
                        });
                    });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().id_salt("mainscroll").show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Client name:")
                                .on_hover_cursor(egui::CursorIcon::Help)
                                .on_hover_text(
                                    "The player name, for identifying instances. Eg. \"Player 1\"",
                                );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.newclientname_display)
                                    .desired_width(250.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Display:")
                                .on_hover_cursor(egui::CursorIcon::Help)
                                .on_hover_text("The display ID to use. Eg. \":30\", \"wayland-2\"");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Backend:")
                                .on_hover_cursor(egui::CursorIcon::Help)
                                .on_hover_text(
                                    "The backend (input sender) to use. Native is recommended.",
                                );
                            //ui.radio_value(&mut self.newbackend_display, Backend::Native, "Native");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Position X:");
                            let mut x = self.newgeometry_display.x.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut x).desired_width(150.0))
                                .changed()
                            {
                                if let Ok(int) = x.parse::<u32>() {
                                    self.newgeometry_display.x = int;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Position Y:");
                            let mut y = self.newgeometry_display.y.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut y).desired_width(150.0))
                                .changed()
                            {
                                if let Ok(int) = y.parse::<u32>() {
                                    self.newgeometry_display.y = int;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            let mut width = self.newgeometry_display.width.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut width).desired_width(150.0))
                                .changed()
                            {
                                if let Ok(int) = width.parse::<u32>() {
                                    self.newgeometry_display.width = int;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Height:");
                            let mut height = self.newgeometry_display.height.to_string();
                            if ui
                                .add(egui::TextEdit::singleline(&mut height).desired_width(150.0))
                                .changed()
                            {
                                if let Ok(int) = height.parse::<u32>() {
                                    self.newgeometry_display.height = int;
                                }
                            }
                        });
                        // TODO: Command
                        ComboBox::from_id_salt("launchmethod")
                            .selected_text(self.newcommand_display.to_string())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.newcommand_display,
                                    CommandType::SteamLaunch {
                                        appid: 0,
                                        settings: crate::types::SteamSettings::Normal,
                                    },
                                    "Steam",
                                );
                                ui.selectable_value(
                                    &mut self.newcommand_display,
                                    CommandType::Manual {
                                        command: "".to_owned(),
                                    },
                                    "Manual",
                                );
                            });
                        match self.newcommand_display.as_mut() {
                            CommandType::Manual { command } => {
                                ui.text_edit_singleline(command);
                            }
                            CommandType::SteamLaunch { appid, settings } => {
                                if ui
                                    .text_edit_singleline(&mut self.newcommand_display_steamid)
                                    .changed()
                                {
                                    if let Ok(pars) = self.newcommand_display_steamid.parse::<u32>()
                                    {
                                        *appid = pars;
                                    } else {
                                        if self.newcommand_display_steamid.contains("/app/") {
                                            if let Some(pars1) = self
                                                .newcommand_display_steamid
                                                .split("/app/")
                                                .nth(1)
                                                .unwrap()
                                                .split("/")
                                                .nth(0)
                                            {
                                                if let Ok(pars2) = pars1.parse::<u32>() {
                                                    *appid = pars2;
                                                    self.newcommand_display_steamid =
                                                        pars1.to_owned();
                                                }
                                            }
                                        }
                                    }
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Proton version:");
                                    ComboBox::from_id_salt("protonversion")
                                        .selected_text(
                                            self.newclient_protonver
                                                .file_name()
                                                .map(|x| x.to_str().unwrap())
                                                .unwrap_or("Unchosen"),
                                        )
                                        .show_ui(ui, |ui| {
                                            // list proton versions

                                            if let Ok(protons) = list_protons() {
                                                self.detectedprotons = protons;
                                            }

                                            for proton in self.detectedprotons.iter() {
                                                ui.selectable_value(
                                                    &mut self.newclient_protonver,
                                                    proton.to_path_buf(),
                                                    proton.file_name().unwrap().to_str().unwrap(),
                                                );
                                            }
                                        });
                                });

                                ui.radio_value(
                                    settings,
                                    crate::types::SteamSettings::Normal,
                                    "Normal",
                                );
                                ui.radio_value(
                                    settings,
                                    crate::types::SteamSettings::Legit,
                                    "Legit",
                                );
                                ui.radio_value(settings, crate::types::SteamSettings::Fake, "Fake");

                                // TODO: remove later, debug only
                                if ui.button("Create launch preferences").clicked() {
                                    if let Err(err) = LaunchPreferences::new(
                                        *appid,
                                        Some(self.newclient_protonver.clone()),
                                    ) {
                                        eprintln!("{:#?}", err);
                                    }
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                self.guistate = GuiState::EditPreset;
                                let devices: Vec<Device> = self
                                    .newdevices_display
                                    .iter()
                                    .filter(|dev| dev.chosen)
                                    .map(|dev| dev.device.clone())
                                    .collect();
                                // create client
                                self.chosenpreset.as_mut().unwrap().clients.push(
                                    Client::new(
                                        self.newclientname_display.to_owned(),
                                        ":1".to_owned(),
                                        &devices,
                                        self.newbackend_display,
                                        self.newgeometry_display,
                                        // TODO: Command
                                        self.newcommand_display.clone(),
                                    )
                                    .unwrap(),
                                );
                                self.newclientname_display = "".to_owned();
                            }
                            if ui.button("Cancel").clicked() {
                                self.guistate = GuiState::EditPreset;
                                self.newclientname_display = "".to_owned();
                            }
                        });
                    });
                });
            });
        });
    }
}

pub fn start() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 430.0]),
        ..Default::default()
    };
    eframe::run_native("Splinux", options, Box::new(|_| Ok(Box::<App>::default()))).unwrap();
}
