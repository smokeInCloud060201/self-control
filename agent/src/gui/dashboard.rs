use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::config::{AppConfig, save_config};

#[derive(PartialEq)]
enum View {
    Dashboard,
    Settings,
}

pub struct DashboardApp {
    pub machine_id: String,
    pub password: Arc<Mutex<String>>,
    pub is_streaming: Arc<Mutex<bool>>,
    pub status: Arc<Mutex<String>>,
    current_view: View,
    settings_passkey: String,
}

impl DashboardApp {
    pub fn new(
        machine_id: String,
        password: Arc<Mutex<String>>,
        is_streaming: Arc<Mutex<bool>>,
        status: Arc<Mutex<String>>,
    ) -> Self {
        let initial_pwd = { password.lock().unwrap().clone() };
        Self {
            machine_id,
            password,
            is_streaming,
            status,
            current_view: View::Dashboard,
            settings_passkey: initial_pwd,
        }
    }

    fn draw_status_dot(&self, ui: &mut egui::Ui, color: egui::Color32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let status = { self.status.lock().unwrap().clone() };
        let is_streaming = { *self.is_streaming.lock().unwrap() };

        egui::CentralPanel::default().show(ctx, |ui| {
            // Top Navigation / Header
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("SelfControl").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Settings").clicked() {
                        self.current_view = if self.current_view == View::Settings { View::Dashboard } else { View::Settings };
                    }
                });
            });

            ui.separator();
            ui.add_space(10.0);

            match self.current_view {
                View::Dashboard => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        
                        let dot_color = if is_streaming {
                            egui::Color32::from_rgb(0, 255, 128) // Green
                        } else if status == "Connected" {
                            egui::Color32::from_rgb(0, 128, 255) // Blue
                        } else {
                            egui::Color32::from_rgb(150, 150, 150) // Gray
                        };

                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 50.0);
                            self.draw_status_dot(ui, dot_color);
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(&status).color(dot_color).strong());
                        });

                        ui.add_space(20.0);

                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::from_gray(30))
                            .rounding(8.0)
                            .show(ui, |ui| {
                                ui.set_width(320.0);
                                ui.add_space(10.0);
                                
                                ui.label(egui::RichText::new("Connection Details").small().weak());
                                ui.add_space(8.0);

                                ui.horizontal(|ui| {
                                    ui.label("Machine ID:");
                                    ui.add_space(5.0);
                                    ui.label(egui::RichText::new(&self.machine_id).monospace().strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Copy").clicked() {
                                            ui.output_mut(|o| o.copied_text = self.machine_id.clone());
                                        }
                                    });
                                });

                                ui.add_space(12.0);

                                ui.horizontal(|ui| {
                                    let current_pwd = { self.password.lock().unwrap().clone() };
                                    ui.label("Passkey:");
                                    ui.add_space(23.0);
                                    ui.label(egui::RichText::new(&current_pwd).monospace().strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Copy").clicked() {
                                            ui.output_mut(|o| o.copied_text = current_pwd);
                                        }
                                    });
                                });
                                
                                ui.add_space(10.0);
                            });

                        ui.add_space(30.0);
                        if is_streaming {
                            ui.label(egui::RichText::new("🔴 Remote session in progress").color(egui::Color32::from_rgb(255, 100, 100)));
                        } else {
                            ui.label(egui::RichText::new("Keep this window open to allow remote access.").weak().small());
                        }
                    });
                }
                View::Settings => {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Settings").strong());
                        ui.add_space(20.0);

                        ui.label("Default Passkey");
                        ui.add_space(5.0);
                        let text_edit = egui::TextEdit::singleline(&mut self.settings_passkey)
                            .hint_text("Set a permanent 6-digit passkey")
                            .desired_width(200.0);
                        
                        ui.add(text_edit);
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Leave empty to generate random each time.").small().weak());

                        ui.add_space(20.0);
                        
                        if ui.button("Save Configuration").clicked() {
                            let mut config = AppConfig::default();
                            if !self.settings_passkey.trim().is_empty() {
                                config.default_passkey = Some(self.settings_passkey.clone());
                            }
                            save_config(&config);
                            
                            // Update current session password
                            let mut pwd = self.password.lock().unwrap();
                            *pwd = self.settings_passkey.clone();
                            
                            self.current_view = View::Dashboard;
                        }
                        
                        ui.add_space(10.0);
                        if ui.button("Back to Dashboard").clicked() {
                            self.current_view = View::Dashboard;
                        }
                    });
                }
            }
        });

        // Repaint periodically to update status
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
