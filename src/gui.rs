use crate::types::{Backend, Client};
use eframe::egui;

struct App {
    // TODO: dont forget to tell the user what's wrong with the arguments
    clientlist: Vec<Client>,
    newclient_display: String,
    newdevices_display: String,
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
            clientlist: vec![],
            newclient_display: ":1".to_owned(),
            newdevices_display: "0,0".to_owned(),
            newbackend_display: Backend::Enigo,
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
                let newclient = Client::new(
                    self.newclient_display.clone(),
                    self.newdevices_display.clone(),
                    self.newbackend_display.clone(),
                );
                self.clientlist.push(newclient);
            }
            ui.group(|ui| {
                self.clientlist.retain_mut(|x| x.is_alive());
                for client in &mut self.clientlist {
                    ui.horizontal(|ui| {
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
            })
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
        viewport: egui::ViewportBuilder::default().with_inner_size([367.0, 432.0]),
        ..Default::default()
    };
    eframe::run_native("Splinux", options, Box::new(|_| Ok(Box::<App>::default()))).unwrap();
}
