//! Stretch foundation hooks for the Agent Client Protocol (`agy acp --stdio`).
//! Strictly observes JSON-RPC / NDJSON events emitted by the external CLI process
//! with zero embedded models or credential access (Unit 7 / Unit 11).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpToolCall {
    pub id: String,
    pub tool: String,
    pub ok: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpArtifact {
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpEvent {
    ToolCallStarted { id: String, tool: String },
    ToolCallFinished { id: String, ok: bool },
    ArtifactCreated { title: String, path: String },
    MessageDelta { text: String },
    StatusChanged { status: String },
}

#[derive(Debug, Clone, Default)]
pub struct AcpCardState {
    pub active_tools: Vec<AcpToolCall>,
    pub recent_artifacts: Vec<AcpArtifact>,
    pub latest_message: String,
    pub latest_status: Option<String>,
}

impl AcpCardState {
    pub fn apply(&mut self, event: &AcpEvent) {
        match event {
            AcpEvent::ToolCallStarted { id, tool } => {
                if !self.active_tools.iter().any(|t| t.id == *id) {
                    self.active_tools.push(AcpToolCall {
                        id: id.clone(),
                        tool: tool.clone(),
                        ok: None,
                    });
                }
            }
            AcpEvent::ToolCallFinished { id, ok } => {
                let _ = ok;
                self.active_tools.retain(|t| t.id != *id);
            }
            AcpEvent::ArtifactCreated { title, path } => {
                self.recent_artifacts.push(AcpArtifact {
                    title: title.clone(),
                    path: path.clone(),
                });
                if self.recent_artifacts.len() > 32 {
                    let drop_count = self.recent_artifacts.len() - 32;
                    self.recent_artifacts.drain(0..drop_count);
                }
            }
            AcpEvent::MessageDelta { text } => {
                self.latest_message.push_str(text);
                if self.latest_message.len() > 2048 {
                    let excess = self.latest_message.len() - 2048;
                    self.latest_message.drain(0..excess);
                }
            }
            AcpEvent::StatusChanged { status } => {
                self.latest_status = Some(status.clone());
            }
        }
    }
}

/// Parse one NDJSON event line emitted by `agy acp --stdio`.
pub fn parse_acp_json_line(line: &str) -> Option<AcpEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let val: Value = serde_json::from_str(trimmed).ok()?;
    let ev_type = val
        .get("type")
        .or_else(|| val.get("event"))
        .and_then(|v| v.as_str())?;

    match ev_type {
        "tool_call_started" => Some(AcpEvent::ToolCallStarted {
            id: val.get("id")?.as_str()?.to_string(),
            tool: val
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string(),
        }),
        "tool_call_finished" => Some(AcpEvent::ToolCallFinished {
            id: val.get("id")?.as_str()?.to_string(),
            ok: val.get("ok").and_then(|v| v.as_bool()).unwrap_or(true),
        }),
        "artifact_created" => Some(AcpEvent::ArtifactCreated {
            title: val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Artifact")
                .to_string(),
            path: val.get("path")?.as_str()?.to_string(),
        }),
        "message_delta" => Some(AcpEvent::MessageDelta {
            text: val.get("text")?.as_str()?.to_string(),
        }),
        "status_changed" => Some(AcpEvent::StatusChanged {
            status: val.get("status")?.as_str()?.to_string(),
        }),
        _ => None,
    }
}
