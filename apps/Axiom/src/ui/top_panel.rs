use eframe::egui;

pub enum TopPanelAction {
    SwitchChannel(String),
    ClearChat,
    ClearScene,
    CopyLog,
    None,
}

pub fn render_top_panel(ui: &mut egui::Ui, active_channel_id: &str) -> TopPanelAction {
    let mut action = TopPanelAction::None;

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.heading("Bevy AI Editor");
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑️ Clear Chat").clicked() {
                action = TopPanelAction::ClearChat;
            }

            ui.add_space(5.0);

            if ui.button("🧹 Clear Scene").clicked() {
                action = TopPanelAction::ClearScene;
            }

            ui.add_space(5.0);

            if ui.button("📋 Copy Log").clicked() {
                action = TopPanelAction::CopyLog;
            }
        });
    });

    action
}
