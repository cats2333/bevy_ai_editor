use crate::tools::Tool;
use crate::types::AsyncMessage;
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct BatchTool {
    tx: Sender<AsyncMessage>,
}

impl BatchTool {
    pub fn new(tx: Sender<AsyncMessage>) -> Self {
        Self { tx }
    }
}

impl Tool for BatchTool {
    fn name(&self) -> String {
        "batch_run".to_string()
    }

    fn description(&self) -> String {
        "Execute multiple tools in parallel (especially useful for spawning multiple sub-agents)."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "batch_run",
                "description": "Execute multiple tools in parallel. Use this to spawn multiple 'task' agents simultaneously.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "tools": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "tool": { "type": "string", "description": "The name of the tool to run" },
                                    "parameters": { "type": "object", "description": "The arguments for the tool" }
                                },
                                "required": ["tool", "parameters"]
                            }
                        }
                    },
                    "required": ["tools"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let tools_list = args
            .get("tools")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing or invalid 'tools' argument"))?;

        // Get registry of all tools, passing the channel
        let tx = self.tx.clone();

        // Wrap tools in Arc for sharing across threads
        let available_tools = Arc::new(crate::tools::get_all_tools(tx.clone()));

        // Use Arc<Mutex<Vec<_>>> to collect results thread-safely
        let results = Arc::new(Mutex::new(Vec::new()));

        // Use Rayon for parallel iteration
        // Explicitly type the closure arguments to help type inference
        tools_list
            .par_iter()
            .enumerate()
            .for_each(|(i, tool_call): (usize, &Value)| {
                let tool_name = tool_call
                    .get("tool")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let params = tool_call.get("parameters").cloned().unwrap_or(json!({}));

                let result_entry =
                    if let Some(tool) = available_tools.iter().find(|t| t.name() == tool_name) {
                        match tool.execute(params) {
                            Ok(output) => json!({
                                "tool": tool_name,
                                "status": "success",
                                "output": output
                            }),
                            Err(e) => json!({
                                "tool": tool_name,
                                "status": "error",
                                "error": e.to_string()
                            }),
                        }
                    } else {
                        json!({
                            "tool": tool_name,
                            "status": "error",
                            "error": format!("Tool '{}' not found", tool_name)
                        })
                    };

                // Lock and push result
                if let Ok(mut guard) = results.lock() {
                    guard.push(result_entry);
                }
            });

        // Extract results
        let final_results = results
            .lock()
            .map_err(|_| anyhow!("Failed to lock results"))?
            .clone();
        Ok(serde_json::to_string_pretty(&final_results)?)
    }
}
