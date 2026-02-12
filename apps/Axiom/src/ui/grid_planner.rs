use base64::prelude::*;
use eframe::egui;
use std::io::Cursor;

pub struct GridPlannerState {
    pub is_open: bool,
    pub drawing_buffer: Option<egui::ColorImage>,
    pub last_pos: Option<egui::Pos2>, // For stroke interpolation
}

impl Default for GridPlannerState {
    fn default() -> Self {
        Self {
            is_open: false,
            drawing_buffer: None,
            last_pos: None,
        }
    }
}

pub enum GridPlannerAction {
    None,
    SendToAI(String, String), // (Prompt, Base64Image)
}

pub fn render_grid_planner(ctx: &egui::Context, state: &mut GridPlannerState) -> GridPlannerAction {
    let mut action = GridPlannerAction::None;

    if !state.is_open {
        return action;
    }

    let mut is_open = state.is_open;
    let mut should_send = false;

    egui::Window::new("🎨 Road Sketch Pad")
        .open(&mut is_open)
        .default_size([600.0, 600.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("❌ Clear Canvas").clicked() {
                    // Reset buffer to WHITE (as requested)
                    if let Some(img) = &mut state.drawing_buffer {
                        for pixel in img.pixels.iter_mut() {
                            *pixel = egui::Color32::WHITE;
                        }
                    }
                }

                ui.add_space(20.0);

                if ui.button("🚀 Generate Road").clicked() {
                    should_send = true;
                }
            });

            ui.separator();

            // --- Canvas Area ---
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());
            let rect = response.rect;

            // Initialize buffer if needed
            let width = size.x as usize;
            let height = size.y as usize;

            if state.drawing_buffer.is_none()
                || state.drawing_buffer.as_ref().unwrap().width() != width
                || state.drawing_buffer.as_ref().unwrap().height() != height
            {
                // Initialize with WHITE background
                state.drawing_buffer =
                    Some(egui::ColorImage::new([width, height], egui::Color32::WHITE));
            }

            // Draw Grid Background (Overlay on buffer or baked in? Baked is better for AI context, but dynamic grid is nicer UI)
            // Actually, if we want AI to see the grid, we should probably rely on the AI inferring grid from context or draw grid into buffer.
            // But let's just draw grid lines on UI for user, and send white canvas with black lines to AI.
            // AI is good at relative coords.
            // However, user requested "Top Down View" which implies the canvas IS the map.

            // Let's render the buffer to screen
            if let Some(img) = &mut state.drawing_buffer {
                let texture =
                    ctx.load_texture("drawing_canvas", img.clone(), egui::TextureOptions::NEAREST);
                painter.image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Draw dynamic grid lines ON TOP for user reference
                let grid_size = 30.0;
                let grid_color = egui::Color32::from_gray(230); // Very faint gray

                let mut x = rect.left();
                while x < rect.right() {
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(1.0, grid_color),
                    );
                    x += grid_size;
                }
                let mut y = rect.top();
                while y < rect.bottom() {
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        egui::Stroke::new(1.0, grid_color),
                    );
                    y += grid_size;
                }

                // Handle Input (Painting)
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        if rect.contains(pos) {
                            let current_pos = pos;
                            if let Some(last) = state.last_pos {
                                let start = last - rect.min;
                                let end = current_pos - rect.min;
                                // Draw RED lines for roads (High contrast)
                                draw_line_in_buffer(img, start, end, egui::Color32::RED, 4.0);
                            }
                            state.last_pos = Some(current_pos);
                        }
                    }
                } else if ui.input(|i| i.pointer.secondary_down()) {
                    // Erase -> Paint WHITE
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        if rect.contains(pos) {
                            let current_pos = pos;
                            if let Some(last) = state.last_pos {
                                let start = last - rect.min;
                                let end = current_pos - rect.min;
                                draw_line_in_buffer(img, start, end, egui::Color32::WHITE, 10.0);
                            }
                            state.last_pos = Some(current_pos);
                        }
                    }
                } else {
                    state.last_pos = None;
                }
            }

            painter.text(
                rect.min + egui::vec2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                "L-Click: Draw Road (Red) | R-Click: Erase",
                egui::FontId::proportional(14.0),
                egui::Color32::BLACK,
            );
        });

    if should_send {
        if let Some(img) = &state.drawing_buffer {
            if let Some(base64_str) = encode_image_to_base64(img) {
                // Determine grid scale
                let w = img.width() as f32;
                let h = img.height() as f32;
                let grid_w = (w / 30.0).round();
                let grid_h = (h / 30.0).round();

                let prompt = format!("Here is a top-down sketch of a road network. \n\n**Image Context**:\n- Background: White with faint grid lines.\n- Roads: **RED LINES**.\n- Scale: 1 grid square = 1x1 unit coordinate.\n- Canvas Center: The geometric center of the image is (0,0).\n- Canvas Size: Approx {:.0}x{:.0} grid units.\n\n**Instructions**:\n1. Ignore the faint grid lines. Focus ONLY on the **RED lines**.\n2. Map the Red lines to grid coordinates based on the background grid.\n3. Recognize shapes: Straight, Turns, Intersections.\n4. Generate the road using `spawn_road_grid` (preferred) or `road_driver`.", grid_w, grid_h);

                action = GridPlannerAction::SendToAI(prompt, base64_str);
                state.is_open = false;
            }
        }
    } else {
        state.is_open = is_open;
    }

    action
}

fn draw_line_in_buffer(
    img: &mut egui::ColorImage,
    start: egui::Vec2,
    end: egui::Vec2,
    color: egui::Color32,
    thickness: f32,
) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let distance = (dx * dx + dy * dy).sqrt();
    let steps = distance.ceil() as i32;

    let w = img.width();
    let h = img.height();

    for i in 0..=steps {
        let t = if steps == 0 {
            0.0
        } else {
            i as f32 / steps as f32
        };
        let x = start.x + dx * t;
        let y = start.y + dy * t;

        let radius = (thickness / 2.0).ceil() as i32;
        for ry in -radius..=radius {
            for rx in -radius..=radius {
                if rx * rx + ry * ry > radius * radius {
                    continue;
                } // Circle brush

                let px = (x as i32 + rx) as usize;
                let py = (y as i32 + ry) as usize;

                if px < w && py < h {
                    img.pixels[py * w + px] = color;
                }
            }
        }
    }
}

fn encode_image_to_base64(img: &egui::ColorImage) -> Option<String> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let mut bytes = Vec::with_capacity(img.pixels.len() * 4);
    for p in &img.pixels {
        bytes.push(p.r());
        bytes.push(p.g());
        bytes.push(p.b());
        bytes.push(p.a());
    }

    if let Some(img_buffer) = image::RgbaImage::from_raw(width, height, bytes) {
        let mut png_bytes: Vec<u8> = Vec::new();
        if let Ok(_) =
            img_buffer.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        {
            return Some(BASE64_STANDARD.encode(&png_bytes));
        }
    }
    None
}
