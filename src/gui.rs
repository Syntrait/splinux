use crate::types::{Backend, Client, Device, DeviceList, get_devices};
use eframe::egui::{self, ScrollArea};

// TODO: do all the launching, save file location spoofing, etc. from the program

struct App {
    // TODO: dont forget to tell the user what's wrong with the arguments
    alertlist: Vec<String>,
    clientlist: Vec<Client>,
    newclient_display: String,
    newdevices_display: Vec<GuiDevice>,
    newbackend_display: Backend,
    aboutwindow_visible: bool,
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
            newclient_display: ":1".to_owned(),
            newdevices_display: vec![],
            newbackend_display: Backend::Native,
            aboutwindow_visible: false,
        }
    }
}

struct GuiDevice {
    device: Device,
    chosen: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().id_salt("mainscroll").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Splinux");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("About").clicked() {
                            self.aboutwindow_visible = !self.aboutwindow_visible;
                        }
                    });
                });
                if ui.button("Create new preset").clicked() {
                    println!("create preset");
                    let devices = DeviceList {
                        devices: get_devices(),
                    };
                    for device in devices.devices.iter() {
                        println!(
                            "device name: {}\ndevice type: {}\ndevice namenum: {}",
                            device.get_name(),
                            device.devicetype,
                            device.namenum.unwrap(),
                        );
                    }
                    println!("{}", toml::to_string(&devices).unwrap());
                }

                if ui.button("Refresh device list").clicked() {
                    let devices: Vec<GuiDevice> = get_devices()
                        .iter()
                        .map(|dev| GuiDevice {
                            device: dev.clone(),
                            chosen: false,
                        })
                        .collect();

                    self.newdevices_display = devices;
                }

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
                    ui.label("Devices:")
                        .on_hover_cursor(egui::CursorIcon::Help)
                        .on_hover_text(
                            "The input devices' IDs, seperated with commas. Eg. \"25,28\"",
                        );
                    /*ui.add(
                        egui::TextEdit::singleline(&mut self.newdevices_display).desired_width(250.0),
                    );*/
                });
                ui.horizontal(|ui| {
                    ui.label("Backend:")
                        .on_hover_cursor(egui::CursorIcon::Help)
                        .on_hover_text("The backend (input sender) to use. Native is recommended.");
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
                ScrollArea::both().id_salt("clientlist").show(ui, |ui| {
                    ui.vertical(|ui| {
                        self.clientlist.retain_mut(|x| x.is_alive());
                        for client in &mut self.clientlist {
                            ui.group(|ui| {
                                ui.label(format!("Client {}", client.pid));
                                ui.group(|ui| {
                                    ui.label(format!("Display: {}", client.display));
                                });
                                ui.group(|ui| {
                                    //ui.label(format!("Devices: {}", client.devices));
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

                if self.newdevices_display.len() != 0 {
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
                }
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
}

pub fn start() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 430.0]),
        ..Default::default()
    };
    eframe::run_native("Splinux", options, Box::new(|_| Ok(Box::<App>::default()))).unwrap();
}
