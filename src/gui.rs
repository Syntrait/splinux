use crate::types::Client;
use eframe::egui;

struct App {
    clientlist: Vec<Client>,
    newclient_display: String,
    newdevices_display: String,
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
                ui.label("Display:");
                ui.add(egui::TextEdit::singleline(&mut self.newclient_display).desired_width(20.0));
            });
            ui.horizontal(|ui| {
                ui.label("Devices:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.newdevices_display).desired_width(50.0),
                );
            });
            if ui.button("+").clicked() {
                let newclient = Client::new(
                    self.newclient_display.clone(),
                    self.newdevices_display.clone(),
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
                ui.label("Version 1.0");
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
