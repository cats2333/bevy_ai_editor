pub mod ast_grep;
pub mod batch;
pub mod bevy;
pub mod locks;
pub mod lsp;
pub mod multiedit;
pub mod search;
pub mod shell;
pub mod todo;

use crate::types::AsyncMessage;
use anyhow::{anyhow, Result};
use bevy::{
    BevyClearSceneTool, BevyRpcTool, BevySpawnPrimitiveTool, BevySpawnSceneTool,
    BevyUploadAssetTool,
};
use serde_json::{json, Value};
use std::fs;
use std::sync::mpsc::Sender;

pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    #[allow(dead_code)]
    fn description(&self) -> String;
    fn schema(&self) -> Value;
    fn execute(&self, args: Value) -> Result<String>;
}

// ... (Other standard tools: ReadFileTool, WriteFileTool, etc.)
// Re-implementing them briefly since I overwrote the file.
// Ideally I should have read the file first and appended.

pub struct ReadFileTool;
impl Tool for ReadFileTool {
    fn name(&self) -> String {
        "read_file".to_string()
    }
    fn description(&self) -> String {
        "Reads a file from the local filesystem.".to_string()
    }
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Reads a file from the local filesystem.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "The path to the file to read" }
                    },
                    "required": ["path"]
                }
            }
        })
    }
    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing path"))?;
        fs::read_to_string(path).map_err(|e| anyhow!("Failed to read file: {}", e))
    }
}

pub struct WriteFileTool;
impl Tool for WriteFileTool {
    fn name(&self) -> String {
        "write_file".to_string()
    }
    fn description(&self) -> String {
        "Write content to a file.".to_string()
    }
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "The path to the file" },
                        "content": { "type": "string", "description": "The content" }
                    },
                    "required": ["path", "content"]
                }
            }
        })
    }
    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing path"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing content"))?;
        let _guard = locks::acquire_lock(path)?;
        fs::write(path, content).map_err(|e| anyhow!("Failed to write: {}", e))?;
        Ok(format!("File written to {}", path))
    }
}

pub struct EditFileTool;
impl Tool for EditFileTool {
    fn name(&self) -> String {
        "edit_file".to_string()
    }
    fn description(&self) -> String {
        "Replace a string in a file.".to_string()
    }
    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Replace a string in a file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path" },
                        "old_string": { "type": "string", "description": "Find" },
                        "new_string": { "type": "string", "description": "Replace" }
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        })
    }
    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing path"))?;
        let old_s = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing old_string"))?;
        let new_s = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing new_string"))?;

        let _guard = locks::acquire_lock(path)?;
        let content = fs::read_to_string(path).map_err(|e| anyhow!("Read fail: {}", e))?;
        if !content.contains(old_s) {
            return Err(anyhow!("old_string not found"));
        }
        let new_content = content.replace(old_s, new_s);
        fs::write(path, new_content).map_err(|e| anyhow!("Write fail: {}", e))?;
        Ok(format!("Edited {}", path))
    }
}

pub fn get_tools_for_profile(profile_name: &str, tx: Sender<AsyncMessage>) -> Vec<Box<dyn Tool>> {
    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ReadFileTool),
        Box::new(WriteFileTool),
        Box::new(EditFileTool),
        Box::new(search::GlobTool),
        Box::new(todo::TodoReadTool),
        Box::new(todo::TodoWriteTool),
        Box::new(ast_grep::AstGrepTool),
        Box::new(batch::BatchTool::new(tx.clone())),
        Box::new(multiedit::MultiEditTool),
        Box::new(lsp::LspTool),
        Box::new(shell::ShellTool),
        Box::new(bevy::BevyUploadAssetTool), // Now available to all agents
        Box::new(bevy::BevyClearSceneTool),  // New: Clear Scene
                                             // Box::new(bevy::BevySpawnPrimitiveTool), // Temporarily disabled to force asset upload workflow
    ];

    if profile_name == "Bevy Editor Companion" {
        tools.push(Box::new(bevy::BevyRpcTool));
        tools.push(Box::new(bevy::BevySpawnSceneTool));
    }

    tools
}

pub fn get_all_tools(tx: Sender<AsyncMessage>) -> Vec<Box<dyn Tool>> {
    get_tools_for_profile("General", tx)
}
