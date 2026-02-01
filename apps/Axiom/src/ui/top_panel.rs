use eframe::egui;

pub enum TopPanelAction {
    SwitchChannel(String),
    ClearChat,
    CopyLog, // New Action
    None,
}

pub fn render_top_panel(ui: &mut egui::Ui, active_channel_id: &str) -> TopPanelAction {
    let mut action = TopPanelAction::None;

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.heading("Bevy AI Editor");
            ui.add_space(20.0);

            // --- Channel Selector (Tabs Style) ---
            // Global
            let global_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new("🌐 Global")
                        .color(if active_channel_id == "global" {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::GRAY
                        })
                        .strong(),
                )
                .fill(if active_channel_id == "global" {
                    egui::Color32::from_rgb(0, 100, 200)
                } else {
                    egui::Color32::TRANSPARENT
                }),
            );

            if global_btn.clicked() {
                action = TopPanelAction::SwitchChannel("global".to_string());
            }

            // Backend
            let backend_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new("🦀 Backend")
                        .color(if active_channel_id == "backend" {
                            egui::Color32::BLACK
                        } else {
                            egui::Color32::GRAY
                        })
                        .strong(),
                )
                .fill(if active_channel_id == "backend" {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::TRANSPARENT
                }),
            );

            if backend_btn.clicked() {
                action = TopPanelAction::SwitchChannel("backend".to_string());
            }

            // Frontend
            let frontend_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new("🎨 Frontend")
                        .color(if active_channel_id == "frontend" {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::GRAY
                        })
                        .strong(),
                )
                .fill(if active_channel_id == "frontend" {
                    egui::Color32::from_rgb(255, 140, 0)
                } else {
                    egui::Color32::TRANSPARENT
                }),
            ); // Dark Orange

            if frontend_btn.clicked() {
                action = TopPanelAction::SwitchChannel("frontend".to_string());
            }

            // Research
            let research_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new("🔍 Research")
                        .color(if active_channel_id == "research" {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::GRAY
                        })
                        .strong(),
                )
                .fill(if active_channel_id == "research" {
                    egui::Color32::from_rgb(100, 0, 150)
                } else {
                    egui::Color32::TRANSPARENT
                }),
            );

            if research_btn.clicked() {
                action = TopPanelAction::SwitchChannel("research".to_string());
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑️ Clear Chat").clicked() {
                action = TopPanelAction::ClearChat;
            }

            ui.add_space(5.0);

            if ui.button("📋 Copy Log").clicked() {
                action = TopPanelAction::CopyLog;
            }

            // Plan / Conductor Button
            /*
            let plan_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new("🎼 Plan")
                        .color(if active_channel_id == "planning" {
                            egui::Color32::BLACK
                        } else {
                            egui::Color32::from_rgb(255, 105, 180) // Hot Pink
                        })
                        .strong(),
                )
                .fill(if active_channel_id == "planning" {
                    egui::Color32::from_rgb(255, 105, 180)
                } else {
                    egui::Color32::TRANSPARENT
                }),
            );

            if plan_btn.clicked() {
                action = TopPanelAction::SwitchChannel("planning".to_string());
            }

            // Multi-Agent Toggle
            let toggle_text = if multi_agent_mode {
                "🔀 Multi-Agent: ON"
            } else {
                "🔀 Multi-Agent: OFF"
            };
            let toggle_color = if multi_agent_mode {
                egui::Color32::GREEN
            } else {
                egui::Color32::GRAY
            };
            if ui
                .button(
                    egui::RichText::new(toggle_text)
                        .color(toggle_color)
                        .strong(),
                )
                .clicked()
            {
                action = TopPanelAction::ToggleMultiAgent;
            }
            */

            // Simulation Trigger Button (One-time use) - HIDDEN as requested
            /*
            if !sim_started {
                 if ui.button(egui::RichText::new("⚡ Sim: New Project").color(egui::Color32::YELLOW)).clicked() {
                    action = TopPanelAction::TriggerSim;
                 }
            }
            */
        });
    });

    action
}
