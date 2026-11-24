use agemax_core::layout::PaneId;
use agemax_core::workspace::{Tab, Workspace};
use std::path::PathBuf;

#[test]
fn test_workspace_and_tab_rename_trims_and_rejects_empty() {
    let pane = PaneId::new();
    let mut ws = Workspace::new("Main", PathBuf::from("C:\\proj"), pane);
    assert!(!ws.rename("   "));
    assert_eq!(ws.name, "Main");

    assert!(ws.rename("  Frontend App  "));
    assert_eq!(ws.name, "Frontend App");

    let mut tab = Tab::new("Terminal 1", pane);
    assert!(!tab.rename(""));
    assert_eq!(tab.title, "Terminal 1");
    assert!(tab.rename("  Build Watcher "));
    assert_eq!(tab.title, "Build Watcher");
}
