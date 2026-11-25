use crate::layout::{LayoutTree, PaneId, TabId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Execution backend hosting shells and agent processes for a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkspaceBackend {
    #[default]
    Local,
    Wsl {
        distro: Option<String>,
        linux_cwd: Option<String>,
    },
    Ssh {
        target: String,
        port: Option<u16>,
        remote_dir: Option<String>,
    },
}

impl WorkspaceBackend {
    pub fn short_badge(&self) -> Option<String> {
        match self {
            Self::Local => None,
            Self::Wsl { distro, .. } => Some(match distro {
                Some(d) => format!("WSL:{d}"),
                None => "WSL".to_string(),
            }),
            Self::Ssh { target, .. } => Some(format!("SSH:{target}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub layout: LayoutTree,
    pub active_pane: PaneId,
    pub zoomed_pane: Option<PaneId>,
}

impl Tab {
    pub fn new(title: impl Into<String>, initial_pane: PaneId) -> Self {
        Self {
            id: TabId::new(),
            title: title.into(),
            layout: LayoutTree::new_leaf(initial_pane),
            active_pane: initial_pane,
            zoomed_pane: None,
        }
    }

    pub fn toggle_zoom(&mut self) {
        if self.zoomed_pane.is_some() {
            self.zoomed_pane = None;
        } else {
            self.zoomed_pane = Some(self.active_pane);
        }
    }

    /// Rename the tab if the trimmed title is non-empty.
    pub fn rename(&mut self, title: impl Into<String>) -> bool {
        let candidate = title.into();
        if let Some(clean) = normalize_label(&candidate) {
            self.title = clean;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub cwd: PathBuf,
    pub git_branch: Option<String>,
    pub listening_ports: Vec<u16>,
    pub profile_label: Option<String>,
    #[serde(default)]
    pub backend: WorkspaceBackend,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
}

impl Workspace {
    pub fn new(name: impl Into<String>, cwd: PathBuf, initial_pane: PaneId) -> Self {
        Self::with_backend(name, cwd, initial_pane, WorkspaceBackend::Local)
    }

    pub fn with_backend(
        name: impl Into<String>,
        cwd: PathBuf,
        initial_pane: PaneId,
        backend: WorkspaceBackend,
    ) -> Self {
        let initial_tab = Tab::new("Terminal 1", initial_pane);
        Self {
            id: WorkspaceId::new(),
            name: name.into(),
            cwd,
            git_branch: None,
            listening_ports: Vec::new(),
            profile_label: None,
            backend,
            tabs: vec![initial_tab],
            active_tab_index: 0,
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    pub fn active_pane_id(&self) -> Option<PaneId> {
        self.active_tab().map(|t| t.active_pane)
    }

    /// Rename the workspace if the trimmed name is non-empty.
    pub fn rename(&mut self, name: impl Into<String>) -> bool {
        let candidate = name.into();
        if let Some(clean) = normalize_label(&candidate) {
            self.name = clean;
            true
        } else {
            false
        }
    }
}

fn normalize_label(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
