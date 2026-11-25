# Unit 14a: Per-Pane Session Restore Path Validation

## 1. Scope
Harden `restore_session_snapshot` in `crates/agemax-app/src/main.rs` so that every `PaneSnapshot.cwd` in `%LOCALAPPDATA%\AgeMax\session.json` is validated against `GeneralConfig.allowed_root_dir` (`REQ-090`), closing the bypass where `ws_snap.cwd` was checked while `pane_snap.cwd` was spawned directly if it existed.

## 2. Requirements Traceability
- **REQ-090** (extends `REQ-081` from Unit 12).

## 3. Design & Deliverables
- Pure helper `resolve_restored_pane_cwd(pane_cwd, ws_dir, allowed_root) -> (PathBuf, Option<String>)` in `crates/agemax-app/src/main.rs`:
  - Returns `(ws_dir, None)` if `pane_cwd` is `None` or does not exist on disk.
  - Calls `validate_workspace_path(path, allowed_root)` when `path.exists()`.
  - Returns `(safe_dir, None)` when valid, or `(ws_dir, Some(err))` when rejected.
- `restore_session_snapshot` wiring:
  - Uses `resolve_restored_pane_cwd` for every pane in `tab_snap.panes` (applying `allowed_root_dir` for `WorkspaceBackend::Local`).
  - Records blocked pane warnings via `notification_store.notify(..., AgentStatus::Failed, ...)` and opens the alerts drawer (`show_notifications_panel = true`).
- Unit tests in `crates/agemax-app/src/main.rs` covering: inside allowed root, outside existing directory, `..` traversal, no allowed root set, and non-existent path.
