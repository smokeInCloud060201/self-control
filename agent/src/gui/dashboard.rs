use eframe::egui;
use crate::config::{AppConfig, save_config};
use crate::app_state::AppState;

#[derive(PartialEq)]
enum View {
    Dashboard,
    Settings,
}

pub struct DashboardApp {
    pub state: AppState,
    current_view: View,
    settings_passkey: String,
    
    // UI State for animations/feedback
    last_copy_id: Option<String>,
    copy_feedback_time: f64,
    streaming_start_time: Option<f64>,
}

impl DashboardApp {
    pub fn new(state: AppState) -> Self {
        let initial_pwd = { state.password_shared.lock().unwrap().clone() };
        Self {
            state,
            current_view: View::Dashboard,
            settings_passkey: initial_pwd,
            last_copy_id: None,
            copy_feedback_time: 0.0,
            streaming_start_time: None,
        }
    }

}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let status = { self.state.status.lock().unwrap().clone() };
        let is_streaming = { *self.state.is_streaming.lock().unwrap() };
        let current_time = ctx.input(|i| i.time);

        // Update streaming timer
        if is_streaming {
            if self.streaming_start_time.is_none() {
                self.streaming_start_time = Some(current_time);
            }
        } else {
            self.streaming_start_time = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header Bar
            egui::TopBottomPanel::top("header").show_inside(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("SelfControl").size(20.0).strong().color(egui::Color32::from_rgb(255, 255, 255)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if self.current_view == View::Settings { "Back" } else { "Settings" };
                        if ui.button(btn_text).clicked() {
                            self.current_view = if self.current_view == View::Settings { View::Dashboard } else { View::Settings };
                        }
                    });
                });
                ui.add_space(5.0);
            });

            ui.add_space(15.0);

            match self.current_view {
                View::Dashboard => {
                    ui.vertical_centered(|ui| {
                        // Status Indicator with Pulsing Animation
                        let dot_color = if is_streaming {
                            egui::Color32::from_rgb(16, 185, 129) // Success Emerald
                        } else if status == "Connected" {
                            egui::Color32::from_rgb(59, 130, 246) // Primary Blue
                        } else {
                            egui::Color32::from_rgb(107, 114, 128) // Gray
                        };

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 60.0);
                            
                            // Pulse circle if streaming
                            let radius = if is_streaming {
                                5.0 + (current_time * 5.0).sin() as f32 * 2.0
                            } else {
                                5.0
                            };
                            
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), radius, dot_color);
                            if is_streaming {
                                ui.painter().circle_stroke(rect.center(), radius + 4.0, egui::Stroke::new(1.0, dot_color.linear_multiply(0.3)));
                            }
                            
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(&status).color(dot_color).size(16.0).strong());
                        });

                        ui.add_space(15.0);

                        // Connection Info Card
                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::from_rgba_premultiplied(40, 40, 40, 200))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                            .rounding(12.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.set_width(340.0);
                                
                                ui.label(egui::RichText::new("CONNECTION DETAILS").size(10.0).color(egui::Color32::from_gray(120)).strong());
                                ui.add_space(12.0);

                                // Machine ID Row
                                ui.horizontal(|ui| {
                                    ui.label("Machine ID");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("📋").on_hover_text("Copy Machine ID").clicked() {
                                            ui.output_mut(|o| o.copied_text = self.state.machine_id.clone());
                                            self.last_copy_id = Some("machine_id".to_string());
                                            self.copy_feedback_time = current_time;
                                        }
                                        ui.label(egui::RichText::new(&self.state.machine_id).monospace().strong().color(egui::Color32::LIGHT_GRAY));
                                    });
                                });

                                if self.last_copy_id.as_deref() == Some("machine_id") && current_time - self.copy_feedback_time < 2.0 {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new("Copied!").size(10.0).color(egui::Color32::from_rgb(16, 185, 129)));
                                    });
                                }

                                ui.add_space(16.0);
                                ui.separator();
                                ui.add_space(16.0);

                                // Passkey Row
                                ui.horizontal(|ui| {
                                    let current_pwd = { self.state.password_shared.lock().unwrap().clone() };
                                    ui.label("Passkey");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("📋").on_hover_text("Copy Passkey").clicked() {
                                            ui.output_mut(|o| o.copied_text = current_pwd.clone());
                                            self.last_copy_id = Some("passkey".to_string());
                                            self.copy_feedback_time = current_time;
                                        }
                                        ui.label(egui::RichText::new(&current_pwd).monospace().strong().color(egui::Color32::LIGHT_GRAY));
                                    });
                                });

                                if self.last_copy_id.as_deref() == Some("passkey") && current_time - self.copy_feedback_time < 2.0 {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new("Copied!").size(10.0).color(egui::Color32::from_rgb(16, 185, 129)));
                                    });
                                }
                            });

                        ui.add_space(25.0);

                        if is_streaming {
                            if let Some(start) = self.streaming_start_time {
                                let elapsed = current_time - start;
                                let mins = (elapsed / 60.0) as i32;
                                let secs = (elapsed % 60.0) as i32;
                                ui.label(egui::RichText::new(format!("🔴 Remote Session: {:02}:{:02}", mins, secs)).color(egui::Color32::from_rgb(255, 100, 100)).strong());
                            }
                        } else {
                            ui.label(egui::RichText::new("Ready for secure remote connection").weak().small());
                        }
                    });
                }
                View::Settings => {
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("AGENT SETTINGS").size(12.0).strong().color(egui::Color32::from_gray(150)));
                        ui.add_space(20.0);

                        ui.label("Persistent Passkey");
                        ui.add_space(8.0);
                        let text_edit = egui::TextEdit::singleline(&mut self.settings_passkey)
                            .hint_text("Set a permanent 6-digit passkey")
                            .desired_width(ui.available_width());
                        
                        let _res = ui.add(text_edit);
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("If set, this password will be used automatically on startup.").small().weak());

                        ui.add_space(30.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("Save and Apply").strong()).clicked() {
                                let mut config = AppConfig::default();
                                if !self.settings_passkey.trim().is_empty() {
                                    config.default_passkey = Some(self.settings_passkey.clone());
                                }
                                save_config(&config);
                                
                                let mut pwd = self.state.password_shared.lock().unwrap();
                                *pwd = self.settings_passkey.clone();
                                
                                self.current_view = View::Dashboard;
                            }
                            
                            if ui.button("Discard Changes").clicked() {
                                let initial_pwd = { self.state.password_shared.lock().unwrap().clone() };
                                self.settings_passkey = initial_pwd;
                                self.current_view = View::Dashboard;
                            }
                        });
                    });
                }
            }
        });

        // Continuous repaint for pulsing animation and feedback timeout
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}
