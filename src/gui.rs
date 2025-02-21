use std::{thread::sleep, time::Duration};

use crate::types::{Backend, Client};
use eframe::egui;
use enigo::{Enigo, Keyboard, Settings};

// TODO: do all the launching, save file location spoofing, etc. from the program

struct App {
    // TODO: dont forget to tell the user what's wrong with the arguments
    alertlist: Vec<String>,
    clientlist: Vec<Client>,
    newclient_display: String,
    newdevices_display: String,
    newbackend_display: Backend,
    newmita_display: bool,
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
            newdevices_display: "0,0".to_owned(),
            newbackend_display: Backend::Enigo,
            newmita_display: false,
            aboutwindow_visible: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
                ui.label("Display:")
                    .on_hover_cursor(egui::CursorIcon::Help)
                    .on_hover_text("The display ID to use. Eg. \"wayland-2\", \":30\" ");
                ui.add(
                    egui::TextEdit::singleline(&mut self.newclient_display).desired_width(250.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Devices:")
                    .on_hover_cursor(egui::CursorIcon::Help)
                    .on_hover_text("The input devices' IDs, seperated with commas. Eg. \"25,28\"");
                ui.add(
                    egui::TextEdit::singleline(&mut self.newdevices_display).desired_width(250.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Backend:")
                    .on_hover_cursor(egui::CursorIcon::Help)
                    .on_hover_text("The backend (input sender) to use. Enigo recommended.");
                ui.radio_value(&mut self.newbackend_display, Backend::Enigo, "Enigo");
                ui.radio_value(&mut self.newbackend_display, Backend::Legacy, "Legacy");
            });
            if ui.button("+").clicked() {
                match Client::new(
                    self.newclient_display.clone(),
                    self.newdevices_display.clone(),
                    self.newbackend_display.clone(),
                    self.newmita_display.clone(),
                ) {
                    Ok(client) => self.clientlist.push(client),
                    Err(err) => self.alertlist.push(err),
                }
            }
            // TODO: programming horror below, remove later
            if ui.button("hello").clicked() {
                sleep(Duration::from_secs(5));
                let mut sett = Settings::default();
                sett.x11_display = None;
                sett.wayland_display = Some("wayland-2".to_owned());
                let mut enigo = Enigo::new(&sett).unwrap();
                enigo
                    .key(enigo::Key::Unicode('a'), enigo::Direction::Press)
                    .unwrap();
                sleep(Duration::from_secs(1));
                enigo
                    .key(enigo::Key::Unicode('a'), enigo::Direction::Release)
                    .unwrap();
            }
            // TODO: make this a scrollable view (and make it look good)
            egui::ScrollArea::both().show(ui, |ui| {
                ui.vertical(|ui| {
                    self.clientlist.retain_mut(|x| x.is_alive());
                    for client in &mut self.clientlist {
                        ui.group(|ui| {
                            ui.label(format!("Client {}", client.pid));
                            ui.group(|ui| {
                                ui.label(format!("Display: {}", client.display));
                            });
                            ui.group(|ui| {
                                ui.label(format!("Devices: {}", client.devices));
                            });
                            ui.group(|ui| {
                                ui.label(format!("Backend: {}", client.backend));
                            });
                            if ui.button("X").clicked() {
                                client.kill();
                            };
                        });
                    }
                });
            });
        });
        if self.aboutwindow_visible {
            egui::Window::new("About").show(ctx, |ui| {
                ui.checkbox(&mut self.newmita_display, "🕯️💝");
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
        viewport: egui::ViewportBuilder::default().with_inner_size([367.0, 432.0]),
        ..Default::default()
    };
    eframe::run_native("Splinux", options, Box::new(|_| Ok(Box::<App>::default()))).unwrap();
}
