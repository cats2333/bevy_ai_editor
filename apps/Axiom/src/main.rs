#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};
use tokio::runtime::Runtime;
use base64::prelude::*;
use std::io::Cursor;
use std::process::Command;
use serde_json::Value;

mod llm;
mod prompts;
mod tools;
mod agent;
mod types;
mod ui;
// mod simulation; // Removed

use crate::llm::{GeminiClient, Message, MessageContent, ContentPart, ImageUrl, StreamEvent, ToolCall, FunctionCall};
use crate::tools::Tool; // Import Tool trait only
use crate::agent::{AgentProfile, get_default_agents};
use crate::types::{AsyncMessage, ChannelState};
use futures_util::StreamExt;

// Import UI modules
use crate::ui::{top_panel, sidebar, input, chat, file_tree};

struct AxiomApp {
    api_key: String,
    
    // Current Active Configuration
    current_profile: AgentProfile,
    available_profiles: Vec<AgentProfile>,
    
    // Channels
    channels: std::collections::HashMap<String, ChannelState>,
    active_channel_id: String,
    
    // Mission Control State (Removed)
    // sub_agents: std::collections::HashMap<String, SubAgentState>,

    // File Tree State
    file_tree_state: ui::file_tree::FileTreeState,

    // Chat & Input State
    input_text: String,
    pending_image: Option<String>, 
    preview_texture: Option<egui::TextureHandle>, 
    clipboard: Option<arboard::Clipboard>,
    
    // App State
    is_loading: bool,
    waiting_for_screenshot: bool,
    client: Option<GeminiClient>,
    // sim_started: bool, // Removed
    // multi_agent_mode: bool, // Removed
    
    // Cache for decoded images
    image_textures: std::collections::HashMap<(usize, usize), egui::TextureHandle>,

    // Async communication
    tx: Sender<AsyncMessage>,
    rx: Receiver<AsyncMessage>,
    rt: Runtime,

    // Conductor State (Removed)
    // active_plan: Option<crate::types::Plan>,
}


impl AxiomApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();
        let font_path = "C:/Windows/Fonts/msyh.ttc";

        if let Ok(data) = std::fs::read(font_path) {
            fonts.font_data.insert(
                "MicrosoftYaHei".to_owned(),
                egui::FontData::from_owned(data),
            );
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "MicrosoftYaHei".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "MicrosoftYaHei".to_owned());
            cc.egui_ctx.set_fonts(fonts);
        }

    let (tx, rx) = channel();
        let rt = Runtime::new().expect("Failed to create Tokio runtime");
        
        // Initialize dotenv
        dotenv::dotenv().ok();
        
        // Remove hardcoded key fallback to prevent leakage
        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();

        let clipboard = arboard::Clipboard::new().ok();

        let mut channels = std::collections::HashMap::new();
        channels.insert("global".to_string(), ChannelState {
            id: "global".to_string(),
            name: "🌐 Global".to_string(),
            history: Vec::new(),
            assigned_agents: vec!["General Assistant".to_string()],
        });
        /*
        channels.insert("backend".to_string(), ChannelState {
            id: "backend".to_string(),
            name: "🦀 Backend".to_string(),
            history: Vec::new(),
            assigned_agents: vec!["Bevy Architect".to_string()],
        });
        channels.insert("frontend".to_string(), ChannelState {
            id: "frontend".to_string(),
            name: "🎨 Frontend".to_string(),
            history: Vec::new(),
            assigned_agents: vec!["Bevy Editor Companion".to_string()],
        });
        channels.insert("research".to_string(), ChannelState {
            id: "research".to_string(),
            name: "🔍 Research".to_string(),
            history: Vec::new(),
            assigned_agents: vec!["Deep Researcher".to_string(), "Pokemon Professor".to_string()],
        });
        channels.insert("planning".to_string(), ChannelState {
            id: "planning".to_string(),
            name: "🎼 Planning".to_string(),
            history: Vec::new(),
            assigned_agents: vec!["Conductor".to_string()],
        });
        */

        Self {
            api_key,
            current_profile: AgentProfile::default(),
            available_profiles: get_default_agents(),
            channels,
            active_channel_id: "global".to_string(),
            // sub_agents: std::collections::HashMap::new(),
            file_tree_state: ui::file_tree::FileTreeState::default(),
            input_text: String::new(),
            pending_image: None,
            preview_texture: None,
            clipboard,
            is_loading: false,
            waiting_for_screenshot: false,
            client: None,
            // sim_started: false,
            // multi_agent_mode: false,
            image_textures: std::collections::HashMap::new(),
            tx,
            rx,
            rt,
            // active_plan: None,
        }
    }

    fn paste_from_clipboard(&mut self, ctx: &egui::Context) -> bool {
        if let Some(clipboard) = &mut self.clipboard {
            match clipboard.get_image() {
                Ok(image_data) => {
                    let width = image_data.width;
                    let height = image_data.height;
                    let bytes = image_data.bytes.into_owned();
                    
                    if let Some(img_buffer) = image::RgbaImage::from_raw(width as u32, height as u32, bytes.clone()) {
                        let mut png_bytes: Vec<u8> = Vec::new();
                        if let Ok(_) = img_buffer.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png) {
                            let base64_string = BASE64_STANDARD.encode(&png_bytes);
                            self.pending_image = Some(base64_string);

                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                [width, height],
                                &bytes,
                            );
                            
                            self.preview_texture = Some(ctx.load_texture(
                                "pending_image",
                                color_image,
                                egui::TextureOptions::default()
                            ));
                            return true;
                        }
                    }
                },
                Err(_) => {}
            }
        }
        false
    }

    fn send_message(&mut self, force: bool) {
        let text = self.input_text.trim().to_string();
        println!("[DEBUG] send_message called. force={}, text_len={}, pending_image={}", force, text.len(), self.pending_image.is_some());
        
        if !force && text.is_empty() && self.pending_image.is_none() { 
            println!("[DEBUG] send_message aborted: empty input and not forced");
            return; 
        }

        let content = if let Some(img_base64) = &self.pending_image {
            let mut parts = Vec::new();
            if !text.is_empty() {
                parts.push(ContentPart {
                    r#type: "text".to_string(),
                    text: Some(text.clone()),
                    image_url: None,
                });
            }
            parts.push(ContentPart {
                r#type: "image_url".to_string(),
                text: None,
                image_url: Some(ImageUrl {
                    url: format!("data:image/png;base64,{}", img_base64),
                }),
            });
            MessageContent::Parts(parts)
        } else {
            MessageContent::Text(text.clone())
        };

        if !text.is_empty() || self.pending_image.is_some() {
            if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                channel.history.push(("Cats2333".to_string(), content.clone()));
            }
        }
        
        self.input_text.clear();
        self.pending_image = None;
        self.preview_texture = None;
        self.is_loading = true;

        // Initialize client if not ready
        if self.client.is_none() {
             match GeminiClient::new(self.api_key.clone(), self.current_profile.model.clone()) {
                Ok(c) => self.client = Some(c),
                Err(e) => {
                    if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                        channel.history.push(("System".to_string(), MessageContent::Text(format!("Failed to init client: {}", e))));
                    }
                    self.is_loading = false;
                    return;
                }
             }
        }

        let client = self.client.as_ref().unwrap().clone();
        let tx = self.tx.clone();
        
        let mut system_prompt = if self.active_channel_id == "planning" {
            crate::prompts::conductor::CONDUCTOR_PROMPT.to_string()
        } else {
            prompts::get_system_prompt(
                &self.current_profile.research_mode,
                &self.current_profile.context_mode,
                &self.current_profile.system_prompt
            )
        };
        
        // --- Multi-Agent Mode Injection ---
        /*
        if self.multi_agent_mode {
            system_prompt.push_str("\n\nCRITICAL: Multi-Agent Mode is ACTIVE. You are an Orchestrator. Do NOT execute complex implementation tasks yourself. You MUST use the 'task' tool to delegate backend, frontend, database, or devops tasks to specialized sub-agents. Your role is to plan and delegate.");
        }
        */
        
        // --- Working Directory Injection ---
        let cwd = self.file_tree_state.root_path.display().to_string();
        system_prompt.push_str(&format!("\n\nCurrent Working Directory: {}\nIMPORTANT: All file operations (read/write/run) should be relative to this directory unless absolute path is specified.", cwd));
        system_prompt.push_str("\n\n**BEVY ASSET RULE**: When generating assets (images, models, etc.) for Bevy, you MUST write them to the `assets/` subdirectory within the working directory. When spawning these assets via `bevy_spawn_scene` or `bevy_spawn`, use the path RELATIVE to the `assets/` folder (e.g., if you wrote 'assets/models/cube.glb', the spawn path is 'models/cube.glb').");
        system_prompt.push_str("\n\n**CRITICAL: BINARY ASSET HANDLING**\nIf the user asks to spawn or use a specific local file (like a .glb, .png, etc.) that is provided in the Context (marked as [BINARY ASSET AVAILABLE]), you **MUST NOT** use `bevy_spawn_primitive` or `bevy_spawn_scene`. \n\nINSTEAD, you **MUST** use the `bevy_upload_asset` tool.\n- `local_path`: Use the absolute path provided in the context (usually in `apps/axiom/resources/...`).\n- `translation`: Use the user's requested position.\n\nExample: User says 'spawn this glb', and context shows `D:/.../dragon.glb`. Call `bevy_upload_asset(local_path='D:/.../dragon.glb', translation=[0,0,0])`. Do NOT try to read the file content or simulate it.");
        
        // Inject Road Engineering Rules
        system_prompt.push_str("\n\n");
        system_prompt.push_str(include_str!("prompts/road_engineer.md"));
        
        let mut messages: Vec<Message> = Vec::new();
        
        if !system_prompt.is_empty() {
            println!("[DEBUG] Adding System Prompt (len={})", system_prompt.len());
            messages.push(Message {
                role: "system".to_string(),
                content: Some(MessageContent::Text(system_prompt)),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let current_history = if let Some(channel) = self.channels.get(&self.active_channel_id) {
            &channel.history
        } else {
            // Should not happen, but safe fallback
            return;
        };

        println!("[DEBUG] Building message history. Count={}", current_history.len());
        for (role, content) in current_history {
            let api_role = match role.as_str() {
                "Cats2333" => "user",
                "System" | "Error" => "system", 
                _ => "assistant", 
            };
            
            if role == "System" || role == "Error" { continue; }

            messages.push(Message {
                role: api_role.to_string(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let profile_name = self.current_profile.name.clone();
        let tools_schema: Vec<Value> = tools::get_tools_for_profile(&profile_name, tx.clone())
            .iter()
            .map(|t| t.schema())
            .collect();

        let rt_handle = self.rt.handle().clone();
        rt_handle.spawn(async move {
            let mut turn_count = 0;
            const MAX_TURNS: i32 = 50;

            loop {
                if turn_count >= MAX_TURNS {
                    let _ = tx.send(AsyncMessage::Error("Max turns exceeded".to_string()));
                    break;
                }
                turn_count += 1;

                match client.chat_completion_stream(messages.clone(), Some(tools_schema.clone())).await {
                    Ok(mut stream) => {
                        let mut full_text = String::new();
                        // let is_planning_channel = messages.iter().any(|m| m.role == "system" && m.content.as_ref().map_or(false, |c| match c { MessageContent::Text(t) => t.contains("Conductor Agent"), _ => false }));
                        
                        struct ToolBuilder {
                            #[allow(dead_code)]
                            index: i32,
                            id: Option<String>,
                            r#type: String, 
                            name: Option<String>,
                            args: String,
                        }
                        let mut tool_buffer: std::collections::HashMap<i32, ToolBuilder> = std::collections::HashMap::new();

                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(StreamEvent::TextChunk(text)) => {
                                    let _ = tx.send(AsyncMessage::StreamText(text.clone()));
                                    full_text.push_str(&text);
                                }
                                Ok(StreamEvent::ToolCallChunk(tc)) => {
                                    let entry = tool_buffer.entry(tc.index).or_insert(ToolBuilder {
                                        index: tc.index,
                                        id: None,
                                        r#type: "function".to_string(),
                                        name: None,
                                        args: String::new(),
                                    });
                                    
                                    if let Some(id) = tc.id { entry.id = Some(id); }
                                    if let Some(t) = tc.r#type { entry.r#type = t; }
                                    if let Some(f) = tc.function {
                                        if let Some(n) = f.name { 
                                            if let Some(curr) = &mut entry.name {
                                                curr.push_str(&n);
                                            } else {
                                                entry.name = Some(n);
                                            }
                                        }
                                        if let Some(a) = f.arguments { entry.args.push_str(&a); }
                                    }
                                }
                                Ok(StreamEvent::Done) => {}
                                Err(e) => {
                                    let _ = tx.send(AsyncMessage::Error(e.to_string()));
                                }
                            }
                        }

                        if !tool_buffer.is_empty() {
                            let mut tool_calls = Vec::new();
                            let mut indices: Vec<i32> = tool_buffer.keys().cloned().collect();
                            indices.sort();
                            
                            for idx in indices {
                                if let Some(builder) = tool_buffer.get(&idx) {
                                    if let Some(name) = &builder.name {
                                        tool_calls.push(ToolCall {
                                            id: builder.id.clone().unwrap_or_else(|| format!("call_{}", idx)),
                                            r#type: builder.r#type.clone(),
                                            function: FunctionCall {
                                                name: name.clone(),
                                                arguments: builder.args.clone(),
                                            },
                                        });
                                    }
                                }
                            }

                            messages.push(Message {
                                role: "assistant".to_string(),
                                content: if !full_text.is_empty() { Some(MessageContent::Text(full_text.clone())) } else { None },
                                tool_calls: Some(tool_calls.clone()),
                                tool_call_id: None,
                            });

                            let all_tools = crate::tools::get_tools_for_profile(&profile_name, tx.clone());
                            for tool_call in tool_calls {
                                let _ = tx.send(AsyncMessage::Log(format!("Executing tool: {} args: {}", tool_call.function.name, tool_call.function.arguments)));
                                
                                let mut result_content = String::new();
                                let mut found = false;
                                
                                for tool in &all_tools {
                                    if tool.name() == tool_call.function.name {
                                        found = true;
                                        match serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments) {
                                            Ok(args_val) => {
                                                match tool.execute(args_val) {
                                                    Ok(res) => result_content = res,
                                                    Err(e) => result_content = format!("Error executing tool: {}", e),
                                                }
                                            },
                                            Err(e) => result_content = format!("Error parsing arguments JSON: {}", e),
                                        }
                                        break;
                                    }
                                }
                                if !found {
                                    result_content = format!("Error: Tool '{}' not found", tool_call.function.name);
                                }

                                messages.push(Message {
                                    role: "tool".to_string(),
                                    content: Some(MessageContent::Text(result_content)),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id),
                                });
                            }
                            continue;
                        }

                        if !full_text.is_empty() {
                            messages.push(Message {
                                role: "assistant".to_string(),
                                content: Some(MessageContent::Text(full_text)),
                                tool_calls: None,
                                tool_call_id: None,
                            });
                            let _ = tx.send(AsyncMessage::Done);
                            break;
                        } else {
                            let _ = tx.send(AsyncMessage::Done);
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AsyncMessage::Error(e.to_string()));
                        break;
                    }
                }
            }
        });
    }
}

impl eframe::App for AxiomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AsyncMessage::StreamText(text) => {
                    self.is_loading = true;
                    
                    let mut append_needed = false;
                    let channel_history = &self.channels.get(&self.active_channel_id).unwrap().history;
                    
                    if let Some((role, content)) = channel_history.last() {
                         if role == "Cats2333" || role == "System" || role == "Error" {
                             append_needed = true;
                         } else if !matches!(content, MessageContent::Text(_)) {
                             append_needed = true;
                         }
                    } else {
                        append_needed = true;
                    }

                    if append_needed {
                         self.channels.get_mut(&self.active_channel_id).unwrap().history.push((self.current_profile.name.clone(), MessageContent::Text(String::new())));
                    }

                    let channel_history_mut = &mut self.channels.get_mut(&self.active_channel_id).unwrap().history;
                    let last_idx = channel_history_mut.len() - 1;
                    if let Some((_, MessageContent::Text(content))) = channel_history_mut.get_mut(last_idx) {
                        content.push_str(&text);
                    }
                }
                AsyncMessage::Done => {
                    self.is_loading = false;
                }
                AsyncMessage::Response(content) => {
                    if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                        channel.history.push((self.current_profile.name.clone(), content));
                    }
                    self.is_loading = false;
                }
                AsyncMessage::Log(text) => {
                     if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                        channel.history.push(("System".to_string(), MessageContent::Text(text)));
                     }
                }
                AsyncMessage::Error(err) => {
                    if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                        channel.history.push(("Error".to_string(), MessageContent::Text(err)));
                    }
                    self.is_loading = false;
                }
            }
            ctx.request_repaint();
        }

        if self.waiting_for_screenshot {
             if self.paste_from_clipboard(ctx) {
                 self.waiting_for_screenshot = false;
             }
        }

        // Layout
        egui::SidePanel::left("file_tree_panel")
            .min_width(200.0)
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                file_tree::render_file_tree(ui, &mut self.file_tree_state);
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                if !self.file_tree_state.selected_files.is_empty() {
                    if ui.button(egui::RichText::new("🚀 Ingest Context").strong().color(egui::Color32::GREEN)).clicked() {
                        let mut targets = Vec::new();
                        let mut references = Vec::new();

                        for path in &self.file_tree_state.selected_files {
                            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                            let is_binary = matches!(extension.as_str(), "glb" | "gltf" | "png" | "jpg" | "jpeg" | "wav" | "ogg" | "mp3");

                            if is_binary {
                                // For binary assets, provide the path but NOT the content
                                // Strong hint to use the upload tool
                                let entry = format!("`{}`: [BINARY ASSET AVAILABLE]. To spawn this in Bevy, you MUST use the 'bevy_upload_asset' tool with this 'local_path'.\n", path.display());
                                references.push(entry);
                            } else if let Ok(content) = std::fs::read_to_string(path) {
                                let is_modify = *self.file_tree_state.selection_modes.get(path).unwrap_or(&false);
                                let entry = format!("`{}`:\n```rust\n{}\n```\n", path.display(), content);
                                
                                if is_modify {
                                    targets.push(entry);
                                } else {
                                    references.push(entry);
                                }
                            }
                        }

                        let mut prompt = String::from("## 📂 Active Context Ingestion\n\n");
                        if !targets.is_empty() {
                            prompt.push_str("### ✏️ TARGETS (Please Modify these):\n");
                            for t in targets { prompt.push_str(&t); prompt.push('\n'); }
                        }
                        if !references.is_empty() {
                            prompt.push_str("### 📖 REFERENCES (Read-Only Context):\n");
                            for r in references { prompt.push_str(&r); prompt.push('\n'); }
                        }
                        prompt.push_str("\n**INSTRUCTION**: Use the Reference files to guide your changes to the Target files.");

                        if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                            channel.history.push(("System".to_string(), MessageContent::Text(prompt)));
                        }
                    }
                }
            });

        /*
        egui::SidePanel::right("right_panel")
            .min_width(150.0)
            .default_width(200.0)
            .show(ctx, |ui| {
                let action = sidebar::render_sidebar(
                    ui, 
                    &self.available_profiles, 
                    &self.current_profile, 
                    &self.active_channel_id, 
                    &self.channels
                );

                match action {
                    sidebar::SidebarAction::SelectProfile(profile) => {
                        self.current_profile = profile;
                        self.client = None;
                    }
                    sidebar::SidebarAction::CopyLog => {
                        let mut log_text = String::new();
                        if let Some(channel) = self.channels.get(&self.active_channel_id) {
                            for (role, content) in &channel.history {
                                let content_str = match content {
                                    MessageContent::Text(t) => t.clone(),
                                    MessageContent::Parts(parts) => {
                                        parts.iter().map(|p| p.text.clone().unwrap_or_default()).collect::<Vec<_>>().join("\n")
                                    }
                                };
                                log_text.push_str(&format!("[{}]: {}\n\n", role, content_str));
                            }
                        }
                        if let Some(clipboard) = &mut self.clipboard {
                            let _ = clipboard.set_text(log_text);
                        }
                    }
                    sidebar::SidebarAction::None => {}
                }
            });
        */

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let action = top_panel::render_top_panel(
                ui, 
                &self.active_channel_id, 
            );
            
            match action {
                top_panel::TopPanelAction::SwitchChannel(id) => {
                    self.active_channel_id = id;
                }
                top_panel::TopPanelAction::ClearChat => {
                    if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                        channel.history.clear();
                    }
                }
                // top_panel::TopPanelAction::ClearScene => {
                //     // Directly execute the Clear Scene tool without involving the LLM
                //     let tool = crate::tools::bevy::BevyClearSceneTool;
                //     let result = match tool.execute(serde_json::Value::Null) {
                //         Ok(msg) => format!("✅ Scene Cleared: {}", msg),
                //         Err(e) => format!("❌ Failed to Clear Scene: {}", e),
                //     };
                //     
                //     if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                //         channel.history.push(("System".to_string(), MessageContent::Text(result)));
                //     }
                // }
                top_panel::TopPanelAction::CopyLog => {
                    let mut log_text = String::new();
                    if let Some(channel) = self.channels.get(&self.active_channel_id) {
                        for (role, content) in &channel.history {
                            let content_str = match content {
                                MessageContent::Text(t) => t.clone(),
                                MessageContent::Parts(parts) => {
                                    parts.iter().map(|p| p.text.clone().unwrap_or_default()).collect::<Vec<_>>().join("\n")
                                }
                            };
                            log_text.push_str(&format!("[{}]: {}\n\n", role, content_str));
                        }
                    }
                    if let Some(clipboard) = &mut self.clipboard {
                        let _ = clipboard.set_text(log_text);
                    }
                }
                top_panel::TopPanelAction::None => {}
            }
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let action = input::render_input_panel(
                ui, 
                &mut self.input_text, 
                self.is_loading, 
                &self.pending_image, 
                &self.preview_texture,
                &self.current_profile
            );

            match action {
                input::InputAction::Send => self.send_message(false),
                input::InputAction::StopLoading => {
                    self.is_loading = false;
                    if let Some(channel) = self.channels.get_mut(&self.active_channel_id) {
                        channel.history.push(("System".to_string(), MessageContent::Text("Stopped by user".to_string())));
                    }
                }
                input::InputAction::RequestScreenshot => {
                    if let Some(clipboard) = &mut self.clipboard {
                        let _ = clipboard.set_text(""); 
                    }
                    let _ = Command::new("cmd")
                        .args(["/C", "start", "ms-screenclip:"])
                        .spawn();
                    self.waiting_for_screenshot = true;
                }
                input::InputAction::ClearPendingImage => {
                    self.pending_image = None;
                    self.preview_texture = None;
                }
                input::InputAction::None => {}
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(channel) = self.channels.get(&self.active_channel_id) {
                        let action = chat::render_chat(
                            ui, 
                            ctx, 
                            &channel.history, 
                            &self.available_profiles, 
                            &mut self.image_textures,
                        );

                        match action {
                            chat::ChatAction::None => {}
                        }
                    }
                });
        });
    }
}

fn main() -> eframe::Result<()> {
    let base_url = std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8045".to_string());
    
    if !base_url.contains("127.0.0.1") && !base_url.contains("localhost") {
        if std::env::var("HTTPS_PROXY").is_err() && std::env::var("https_proxy").is_err() {
            println!("Setting default HTTPS_PROXY to http://127.0.0.1:17890");
            unsafe {
                std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:17890");
            }
        }
    } else {
        println!("Targeting localhost ({}). Skipping default proxy setup.", base_url);
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Bevy AI Editor"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Bevy AI Editor",
        options,
        Box::new(|cc| Ok(Box::new(AxiomApp::new(cc)))),
    )
}
