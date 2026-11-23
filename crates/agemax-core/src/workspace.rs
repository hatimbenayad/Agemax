use crate::layout::{LayoutTree, PaneId, TabId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub cwd: PathBuf,
    pub git_branch: Option<String>,
    pub listening_ports: Vec<u16>,
    pub profile_label: Option<String>,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
}

impl Workspace {
    pub fn new(name: impl Into<String>, cwd: PathBuf, initial_pane: PaneId) -> Self {
        let initial_tab = Tab::new("Terminal 1", initial_pane);
        Self {
            id: WorkspaceId::new(),
            name: name.into(),
            cwd,
            git_branch: None,
            listening_ports: Vec::new(),
            profile_label: None,
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
}
