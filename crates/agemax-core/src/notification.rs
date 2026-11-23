use crate::layout::{PaneId, WorkspaceId};
use crate::status::AgentStatus;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationItem {
    pub id: Uuid,
    pub pane_id: PaneId,
    pub workspace_id: WorkspaceId,
    pub status: AgentStatus,
    pub message: String,
    pub timestamp: SystemTime,
    pub read: bool,
}

#[derive(Debug, Default)]
pub struct NotificationStore {
    items: Vec<NotificationItem>,
}

impl NotificationStore {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn notify(
        &mut self,
        workspace_id: WorkspaceId,
        pane_id: PaneId,
        status: AgentStatus,
        message: impl Into<String>,
    ) {
        let item = NotificationItem {
            id: Uuid::new_v4(),
            pane_id,
            workspace_id,
            status,
            message: message.into(),
            timestamp: SystemTime::now(),
            read: false,
        };
        self.items.push(item);
    }

    pub fn mark_pane_read(&mut self, pane_id: PaneId) {
        for item in &mut self.items {
            if item.pane_id == pane_id {
                item.read = true;
            }
        }
    }

    pub fn unread_count_for_workspace(&self, workspace_id: WorkspaceId) -> usize {
        self.items
            .iter()
            .filter(|i| i.workspace_id == workspace_id && !i.read)
            .count()
    }

    pub fn latest_unread_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Option<&NotificationItem> {
        self.items
            .iter()
            .rev()
            .find(|i| i.workspace_id == workspace_id && !i.read)
    }

    pub fn oldest_unread_pane(&self) -> Option<(WorkspaceId, PaneId)> {
        self.items
            .iter()
            .find(|i| !i.read)
            .map(|i| (i.workspace_id, i.pane_id))
    }

    pub fn unread_items(&self) -> Vec<&NotificationItem> {
        self.items.iter().filter(|i| !i.read).collect()
    }
}
