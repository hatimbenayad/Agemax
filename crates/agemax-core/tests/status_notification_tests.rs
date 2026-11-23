use agemax_core::layout::{PaneId, WorkspaceId};
use agemax_core::notification::NotificationStore;
use agemax_core::status::AgentStatus;

#[test]
fn test_agent_status_properties() {
    assert!(!AgentStatus::Idle.is_attention_needed());
    assert!(!AgentStatus::Working.is_attention_needed());
    assert!(AgentStatus::WaitingForInput.is_attention_needed());
    assert!(AgentStatus::Done.is_attention_needed());
    assert!(AgentStatus::Failed.is_attention_needed());
}

#[test]
fn test_notification_store_lifecycle() {
    let mut store = NotificationStore::new();
    let ws_id = WorkspaceId::new();
    let p1 = PaneId::new();
    let p2 = PaneId::new();

    store.notify(ws_id, p1, AgentStatus::WaitingForInput, "Confirm code edit");
    store.notify(ws_id, p2, AgentStatus::Done, "Build succeeded");

    assert_eq!(store.unread_count_for_workspace(ws_id), 2);
    assert_eq!(store.oldest_unread_pane(), Some((ws_id, p1)));

    // Mark p1 read on user input
    store.mark_pane_read(p1);
    assert_eq!(store.unread_count_for_workspace(ws_id), 1);
    assert_eq!(store.oldest_unread_pane(), Some((ws_id, p2)));

    // Mark p2 read
    store.mark_pane_read(p2);
    assert_eq!(store.unread_count_for_workspace(ws_id), 0);
    assert_eq!(store.oldest_unread_pane(), None);
}
