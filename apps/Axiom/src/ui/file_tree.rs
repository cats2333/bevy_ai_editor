use eframe::egui;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileTreeState {
    pub root_path: PathBuf,
    pub input_path: String, // Buffer for the path input box
    pub selected_files: HashSet<PathBuf>,
    pub expanded_paths: HashSet<PathBuf>,
    // New: Track selection mode for each file (Modify vs Reference)
    // If a file is in 'selected_files', we check this map.
    // true = Modify (Target), false = Reference (Context only)
    pub selection_modes: std::collections::HashMap<PathBuf, bool>,
}

impl Default for FileTreeState {
    fn default() -> Self {
        // Start at apps/axiom/assets by default for better UX
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_root = cwd.join("apps").join("axiom").join("assets");

        // Fallback to CWD if that specific path doesn't exist
        let root = if default_root.exists() {
            default_root
        } else {
            cwd.clone()
        };

        let mut expanded = HashSet::new();
        // If we are deep inside, expand the models folder
        let models_path = root.join("models");
        if models_path.exists() {
            expanded.insert(models_path);
        }

        Self {
            root_path: root.clone(),
            input_path: root.to_string_lossy().to_string(),
            selected_files: HashSet::new(),
            expanded_paths: expanded,
            selection_modes: std::collections::HashMap::new(),
        }
    }
}

pub fn render_file_tree(ui: &mut egui::Ui, state: &mut FileTreeState) {
    ui.heading("📂 Project Files");
    ui.separator();

    // Root Path Input Area
    ui.horizontal(|ui| {
        // ui.label("Root:");
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.input_path).desired_width(180.0), // Fixed width
        );

        if ui.button("🔄").on_hover_text("Update Root Path").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            let new_path = PathBuf::from(&state.input_path);
            if new_path.exists() && new_path.is_dir() {
                state.root_path = new_path;
                state.selected_files.clear(); // Clear selections on root change
                                              // state.expanded_paths.clear(); // UX Improvement: Don't clear expansions on root refresh
            }
        }

        // Add Directory Picker Button using `rfd`
        if ui
            .button("📂")
            .on_hover_text("Select Workspace Folder")
            .clicked()
        {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                state.root_path = folder.clone();
                state.input_path = folder.to_string_lossy().to_string();
                state.selected_files.clear();
                state.expanded_paths.clear();
            }
        }
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("file_tree_scroll")
        .max_height(ui.available_height() - 100.0) // Leave space for buttons at bottom
        .show(ui, |ui| {
            let root = state.root_path.clone();
            if root.exists() {
                render_path_node(ui, &root, state);
            } else {
                ui.label("Invalid root path");
            }
        });
}

fn render_path_node(ui: &mut egui::Ui, path: &Path, state: &mut FileTreeState) {
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let is_dir = path.is_dir();
    let path_buf = path.to_path_buf();

    if is_dir {
        // Folder Logic
        let is_expanded = state.expanded_paths.contains(&path_buf);
        let icon = if is_expanded { "📂" } else { "📁" };

        ui.horizontal(|ui| {
            let btn = ui.button(format!("{} {}", icon, file_name));
            if btn.clicked() {
                if is_expanded {
                    state.expanded_paths.remove(&path_buf);
                } else {
                    state.expanded_paths.insert(path_buf.clone());
                }
            }
        });

        if is_expanded {
            ui.indent(path.to_string_lossy(), |ui| {
                if let Ok(entries) = fs::read_dir(path) {
                    let mut entries_vec: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                    entries_vec.sort_by(|a, b| {
                        let a_dir = a.path().is_dir();
                        let b_dir = b.path().is_dir();
                        if a_dir == b_dir {
                            a.file_name().cmp(&b.file_name())
                        } else {
                            b_dir.cmp(&a_dir)
                        }
                    });

                    for entry in entries_vec {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') && name != "target" {
                            render_path_node(ui, &entry.path(), state);
                        }
                    }
                }
            });
        }
    } else {
        // File Logic
        ui.horizontal(|ui| {
            // 1. Selection Checkbox
            let is_selected = state.selected_files.contains(&path_buf);
            let mut checked = is_selected;

            if ui.checkbox(&mut checked, &file_name).changed() {
                if checked {
                    state.selected_files.insert(path_buf.clone());
                    // Default to Reference mode (safer) if new
                    if !state.selection_modes.contains_key(&path_buf) {
                        state.selection_modes.insert(path_buf.clone(), false);
                    }
                } else {
                    state.selected_files.remove(&path_buf);
                    state.selection_modes.remove(&path_buf);
                }
            }

            // 2. Mode Toggle (Only show if selected)
            if is_selected {
                let is_modify = *state.selection_modes.get(&path_buf).unwrap_or(&false);

                // Toggle Button: 📖 (Read) vs ✏️ (Write)
                let (icon, color, tooltip) = if is_modify {
                    ("✏️", egui::Color32::LIGHT_RED, "Target for Modification")
                } else {
                    ("📖", egui::Color32::LIGHT_BLUE, "Reference Context Only")
                };

                if ui
                    .add(egui::Button::new(egui::RichText::new(icon).color(color)).frame(false))
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    state.selection_modes.insert(path_buf.clone(), !is_modify);
                }
            }
        });
    }
}
