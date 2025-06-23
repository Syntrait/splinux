use crate::types::{Backend, Client, Device, DeviceList, GuiState, Preset, get_devices};
use eframe::egui::{self, ScrollArea, TextEdit};

// TODO: do all the launching, save file location spoofing, etc. from the program

struct App {
    // TODO: dont forget to tell the user what's wrong with the arguments
    alertlist: Vec<String>,
    clientlist: Vec<Client>,
    guistate: GuiState,
    newclient_display: String,
    newpresetname_display: String,
    newclientname_display: String,
    newdevices_display: Vec<GuiDevice>,
    newbackend_display: Backend,
    aboutwindow_visible: bool,
    presetlist: Vec<Preset>,
    chosenpreset: Option<Preset>,
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
            alertlist: vec![],
            clientlist: vec![],
            guistate: GuiState::MainMenu,
            newclient_display: ":1".to_owned(),
            newpresetname_display: "".to_owned(),
            newclientname_display: "".to_owned(),
            newdevices_display: get_ui_devices(),
            newbackend_display: Backend::Native,
            aboutwindow_visible: false,
            presetlist: vec![],
            chosenpreset: None,
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
            GuiState::EditClient => self.render_editclient(&ctx),
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
            if ui.button("Create new preset").clicked() {
                let preset = Preset::new(self.newpresetname_display.to_owned(), vec![]);
                self.presetlist.push(preset);
            }

            ui.vertical(|ui| {
                ScrollArea::both().id_salt("presetlist").show(ui, |ui| {
                    ui.group(|ui| {
                        for preset in self.presetlist.iter() {
                            ui.group(|ui| {
                                if ui.button("Choose").clicked() {
                                    self.chosenpreset = Some(preset.clone());
                                    self.guistate = GuiState::EditPreset;
                                }
                                ui.label(format!("Preset: {}", preset.name));
                            });
                        }
                    });
                });
            });
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

    fn render_editclient(&mut self, ctx: &eframe::egui::Context) {
        egui::SidePanel::right("devicelistpanel")
            .default_width(400.0)
            .show(ctx, |ui| {
                if ui.button("Refresh device list").clicked() {
                    self.newdevices_display = get_ui_devices();
                }
                if self.newdevices_display.len() != 0 {
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
                            ui.label("Name:")
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
                            ui.add(
                                egui::TextEdit::singleline(&mut self.newclient_display)
                                    .desired_width(250.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Backend:")
                                .on_hover_cursor(egui::CursorIcon::Help)
                                .on_hover_text(
                                    "The backend (input sender) to use. Native is recommended.",
                                );
                            ui.radio_value(&mut self.newbackend_display, Backend::Native, "Native");
                            ui.radio_value(&mut self.newbackend_display, Backend::Enigo, "Enigo");
                        });
                        let add_button = ui.button("+");
                        if add_button.clicked() {
                            // lose focus, so space/enter doesn't spam click the add button
                            add_button.surrender_focus();
                            let devices: Vec<Device> = self
                                .newdevices_display
                                .iter()
                                .map(|gd| gd.device.clone())
                                .collect();

                            match Client::new(
                                self.newclientname_display.clone(),
                                self.newclient_display.clone(),
                                &devices,
                                self.newbackend_display.clone(),
                            ) {
                                Ok(client) => {
                                    self.clientlist.push(client);
                                    // refresh client list
                                    ctx.request_repaint();
                                }
                                Err(err) => self.alertlist.push(err.to_string()),
                            }
                        }

                        if self.clientlist.len() != 0 {
                            ScrollArea::both().id_salt("clientlist").show(ui, |ui| {
                                ui.vertical(|ui| {
                                    self.clientlist.retain_mut(|x| x.is_alive());
                                    for client in &mut self.clientlist {
                                        ui.group(|ui| {
                                            ui.label(format!("Client {}", client.pid));
                                            ui.group(|ui| {
                                                ui.label(format!("Name: {}", client.name));
                                            });
                                            ui.group(|ui| {
                                                ui.label(format!("Display: {}", client.display));
                                            });
                                            ui.group(|ui| {
                                                ui.label(format!("Backend: {}", client.backend));
                                            });
                                            if ui.button("X").clicked() {
                                                client.kill();
                                                // refresh client list
                                                ctx.request_repaint();
                                            };
                                        });
                                    }
                                });
                            });
                        }
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
