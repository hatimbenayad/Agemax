# Unit 2: Splits, Tabs, Workspaces & Vertical Sidebar

## 1. Scope
Implement the full multi-workspace and multi-pane hierarchy: collapsible vertical sidebar, workspaces, tabs, binary layout tree splits (horizontal/vertical), splitter resizing, directional focus navigation, pane zoom, and asynchronous workspace metadata discovery (Git branch, cwd, listening ports).

## 2. Requirements Traceability
- **REQ-011**: Collapsible vertical sidebar listing Workspaces.
- **REQ-012**: Workspace metadata (name, Git branch, cwd, listening ports, status).
- **REQ-013**: Throttled asynchronous background metadata discovery.
- **REQ-014**: Workspaces containing Tabs.
- **REQ-015**: Binary layout tree for horizontal/vertical pane splits.
- **REQ-016**: Resizable split ratios via splitter dragging.
- **REQ-017**: Property-tested layout tree operations.
- **REQ-018**: Directional focus navigation (`FocusLeft`, `FocusRight`, `FocusUp`, `FocusDown`).
- **REQ-019**: Single-pane zoom toggle (`Zoom`).
- **REQ-020**: Clean pane closure and tab collapse.

## 3. Interfaces & Data Structures
```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub cwd: PathBuf,
    pub git_branch: Option<String>,
    pub listening_ports: Vec<u16>,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
}

pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub layout: LayoutTree,
    pub zoomed_pane: Option<PaneId>,
}

pub enum LayoutTree {
    Leaf(PaneId),
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutTree>,
        second: Box<LayoutTree>,
    },
}
```

## 4. Acceptance Criteria
1. The sidebar lists all active workspaces and collapses/expands cleanly (`Ctrl+B`).
2. Splitting a pane right (`Ctrl+Shift+D`) or down (`Ctrl+Shift+R`) creates an independent terminal surface with split ratio 0.5.
3. Dragging the splitter dynamically resizes adjacent panes.
4. Directional navigation shortcuts (`Alt+Left`, `Alt+Right`, etc.) transfer focus to adjacent panes.
5. Zoom shortcut (`Ctrl+Shift+Enter`) maximizes the focused pane to 100% of the tab area; pressing it again restores the split layout.
6. Git branch and listening ports are discovered in background worker threads without dropping UI frame rates.

## 5. Test Plan
- Property-based tests (`proptest`) for `LayoutTree`:
  - Split insertion preserves total bounding rectangle.
  - Leaf removal collapses cleanly to sibling.
  - Arbitrary sequences of splits, closes, and resizes never produce zero-size or overlapping panes.
- Unit tests for directional focus calculation (Manhattan distance between pane centers).
- Background watcher test verifying Git branch updates when branch changes.
