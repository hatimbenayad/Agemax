use crate::layout::{LayoutTree, PaneId, SplitDirection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("I/O error during session persistence: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse session JSON: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaneSnapshot {
    pub id: Uuid,
    pub cwd: PathBuf,
    pub agent_conversation_id: Option<String>,
    pub last_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LayoutSnapshot {
    Leaf(Uuid),
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutSnapshot>,
        second: Box<LayoutSnapshot>,
    },
}

impl From<&LayoutTree> for LayoutSnapshot {
    fn from(tree: &LayoutTree) -> Self {
        match tree {
            LayoutTree::Leaf(pane_id) => LayoutSnapshot::Leaf(pane_id.0),
            LayoutTree::Split {
                direction,
                ratio,
                first,
                second,
            } => LayoutSnapshot::Split {
                direction: *direction,
                ratio: *ratio,
                first: Box::new(first.as_ref().into()),
                second: Box::new(second.as_ref().into()),
            },
        }
    }
}

impl From<&LayoutSnapshot> for LayoutTree {
    fn from(snap: &LayoutSnapshot) -> Self {
        match snap {
            LayoutSnapshot::Leaf(id) => LayoutTree::Leaf(PaneId(*id)),
            LayoutSnapshot::Split {
                direction,
                ratio,
                first,
                second,
            } => LayoutTree::Split {
                direction: *direction,
                ratio: *ratio,
                first: Box::new(first.as_ref().into()),
                second: Box::new(second.as_ref().into()),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabSnapshot {
    pub name: String,
    pub active_pane_id: Uuid,
    pub layout: LayoutSnapshot,
    pub panes: Vec<PaneSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceSnapshot {
    pub id: Uuid,
    pub name: String,
    pub cwd: PathBuf,
    pub profile: Option<String>,
    pub active_tab_index: usize,
    pub tabs: Vec<TabSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSnapshot {
    pub version: u32,
    pub active_workspace_index: usize,
    pub workspaces: Vec<WorkspaceSnapshot>,
}

impl SessionSnapshot {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new(active_workspace_index: usize, workspaces: Vec<WorkspaceSnapshot>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            active_workspace_index,
            workspaces,
        }
    }

    /// Save the snapshot to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), SessionError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load and deserialize a session snapshot from a JSON file.
    pub fn load_from_file(path: &Path) -> Result<Self, SessionError> {
        let content = std::fs::read_to_string(path)?;
        let snapshot: Self = serde_json::from_str(&content)?;
        Ok(snapshot)
    }

    /// Returns the default session file path under `%LOCALAPPDATA%\AgeMax\session.json`.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("com", "agemax", "AgeMax")
            .map(|p| p.data_local_dir().join("session.json"))
            .unwrap_or_else(|| PathBuf::from(".agemax").join("session.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serialization_roundtrip() {
        let pane_id = Uuid::new_v4();
        let ws_id = Uuid::new_v4();

        let tab = TabSnapshot {
            name: "Terminal 1".to_string(),
            active_pane_id: pane_id,
            layout: LayoutSnapshot::Leaf(pane_id),
            panes: vec![PaneSnapshot {
                id: pane_id,
                cwd: PathBuf::from("C:\\Projects\\test"),
                agent_conversation_id: Some("conv-abc-123".to_string()),
                last_command: None,
            }],
        };

        let ws = WorkspaceSnapshot {
            id: ws_id,
            name: "Main Workspace".to_string(),
            cwd: PathBuf::from("C:\\Projects\\test"),
            profile: Some("default".to_string()),
            active_tab_index: 0,
            tabs: vec![tab],
        };

        let snapshot = SessionSnapshot::new(0, vec![ws]);

        let json = serde_json::to_string(&snapshot).unwrap();
        let loaded: SessionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, loaded);
    }
}
