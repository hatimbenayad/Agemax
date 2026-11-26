use agemax_core::layout::{LayoutTree, PaneId};
use agemax_core::model::{Point, Rect};
use agemax_core::session::WorkspaceSnapshot;
use agemax_core::workspace::{Workspace, WorkspaceBackend};
use std::path::PathBuf;

#[test]
fn test_workspace_backend_default_and_serde() {
    let local = WorkspaceBackend::default();
    assert_eq!(local, WorkspaceBackend::Local);

    let ws_ssh = Workspace::with_backend(
        "Prod SSH",
        PathBuf::from("."),
        PaneId::new(),
        WorkspaceBackend::Ssh {
            target: "dev@example.com".to_string(),
            port: Some(2222),
            remote_dir: Some("/srv/app".to_string()),
        },
    );
    assert!(matches!(ws_ssh.backend, WorkspaceBackend::Ssh { .. }));

    // Ensure legacy session.json without `backend` deserializes cleanly as Local.
    let legacy_json = serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "name": "Legacy",
        "cwd": "C:\\Projects",
        "profile": null,
        "active_tab_index": 0,
        "tabs": []
    });
    let snap: WorkspaceSnapshot = serde_json::from_value(legacy_json).unwrap();
    assert_eq!(snap.backend, WorkspaceBackend::Local);
    assert_eq!(snap.locked_root, None);
    assert_eq!(agemax_core::session::SessionSnapshot::CURRENT_VERSION, 1);

    let ws_local = Workspace::new("Locked", PathBuf::from("C:\\Projects\\A"), PaneId::new());
    assert_eq!(ws_local.locked_root, None);

    let locked_json = serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "name": "LockedSnap",
        "cwd": "C:\\Projects\\A",
        "profile": null,
        "locked_root": "C:\\Projects\\A",
        "active_tab_index": 0,
        "tabs": []
    });
    let locked_snap: WorkspaceSnapshot = serde_json::from_value(locked_json).unwrap();
    assert_eq!(
        locked_snap.locked_root,
        Some(PathBuf::from("C:\\Projects\\A"))
    );
}

#[test]
fn test_layout_create_team_grid_balanced_partitions() {
    let coord = PaneId::new();
    let w1 = PaneId::new();
    let w2 = PaneId::new();
    let w3 = PaneId::new();

    let tree = LayoutTree::create_team_grid(coord, &[w1, w2, w3]);
    let panes = tree.panes();
    assert_eq!(panes.len(), 4);
    assert!(panes.contains(&coord));
    assert!(panes.contains(&w1));
    assert!(panes.contains(&w2));
    assert!(panes.contains(&w3));

    let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(1000.0, 800.0));
    let rects = tree.compute_rects(bounds);
    assert_eq!(rects.len(), 4);

    let total_area: f32 = rects.values().map(|r| r.width() * r.height()).sum();
    assert!((total_area - 800_000.0).abs() < 1.0);
    for r in rects.values() {
        assert!(r.width() > 100.0 && r.height() > 100.0);
    }
}
