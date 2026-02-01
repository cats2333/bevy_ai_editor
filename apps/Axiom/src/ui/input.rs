use crate::agent::AgentProfile;
use eframe::egui;

pub enum InputAction {
    Send,
    StopLoading,
    RequestScreenshot,
    ClearPendingImage,
    None,
}

pub fn render_input_panel(
    ui: &mut egui::Ui,
    input_text: &mut String,
    is_loading: bool,
    pending_image: &Option<String>,
    preview_texture: &Option<egui::TextureHandle>,
    current_profile: &AgentProfile,
) -> InputAction {
    let mut action = InputAction::None;

    // Add some spacing at the top
    ui.add_space(5.0);

    ui.vertical(|ui| {
        // Show pending image preview if any
        let mut should_clear = false;
        if let Some(texture) = preview_texture {
            ui.horizontal(|ui| {
                ui.label("Image attached");
                // Scale height to 80px (User requested smaller)
                let size = texture.size_vec2();
                let scale = 80.0 / size.y;
                ui.image((texture.id(), size * scale));
                if ui.button("❌").clicked() {
                    should_clear = true;
                }
            });
        } else if pending_image.is_some() {
            // Fallback if we have base64 but no texture
            ui.horizontal(|ui| {
                ui.label("Image attached (No preview)");
                if ui.button("❌").clicked() {
                    should_clear = true;
                }
            });
        }

        if should_clear {
            action = InputAction::ClearPendingImage;
        }

        ui.horizontal(|ui| {
            if ui.button("➕").clicked() {
                action = InputAction::RequestScreenshot;
            }

            // We capture focus lost + enter key for send
            let text_edit = ui.add(
                egui::TextEdit::singleline(input_text).desired_width(ui.available_width() - 80.0),
            );

            if is_loading {
                ui.spinner();
                if ui
                    .button(egui::RichText::new("⏹").color(egui::Color32::RED))
                    .clicked()
                {
                    action = InputAction::StopLoading;
                }
            } else {
                let send_btn = ui.button(egui::RichText::new("▶").color(egui::Color32::GREEN));
                if send_btn.clicked()
                    || (text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    action = InputAction::Send;
                    // Refocus input after sending
                    ui.memory_mut(|mem| mem.request_focus(text_edit.id));
                }
            }
        });

        // Identity label moved to bottom
        ui.label(
            egui::RichText::new(format!("🎭 Identity: {}", current_profile.name))
                .small()
                .weak(),
        );
    });

    // Add some spacing at the bottom to lift it up
    ui.add_space(10.0);

    action
}
