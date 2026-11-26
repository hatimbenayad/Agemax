# Unit 14b: Independent Per-Workspace Folder Locks

## 1. Scope
Replace the single global-fence UX with independent per-workspace folder locks (`REQ-091`–`REQ-094`, `ADR-008`) while preserving `GeneralConfig.allowed_root_dir` as an optional App-Wide Safe Root Ceiling and keeping `enforce_agy_sandbox` global only. Excludes `.agents/hooks.json` scaffolding (deferred to optional Unit 14d).

## 2. Requirements Traceability
- **REQ-091**, **REQ-092**, **REQ-093**, **REQ-094** (`specs/01-requirements.md`, `adr/008-per-workspace-folder-locks.md`).

## 3. Design & Deliverables
- `crates/agemax-core/src/workspace.rs` & `crates/agemax-core/src/session.rs`:
  - `#[serde(default)] pub locked_root: Option<PathBuf>` on `Workspace` and `WorkspaceSnapshot` (`SessionSnapshot::CURRENT_VERSION` stays `1`).
- `crates/agemax-app/src/chrome_ui.rs`:
  - `ToolbarAction::ToggleActiveWorkspaceLock` for one-click folder-bar pill toggling.
  - `render_workspace_folder_bar` accepts `ws_locked_root: Option<&Path>` and `global_allowed_root: Option<&Path>` with honest wording tooltips.
  - `NewWorkspaceModal` & `NewWorkspaceRequest` gain `lock_to_folder: bool` (default `false`) with honest wording helper text.
- `crates/agemax-app/src/sidebar.rs`:
  - `SidebarAction::ToggleWorkspaceLock(usize)`, lock icon badge on locked workspace cards, and right-click context menu item (`Lock Workspace to Folder` / `Unlock Workspace`).
- `crates/agemax-app/src/onboarding_settings.rs`:
  - Relabel `SettingsTab::Safety` global fence section as `"App-Wide Safe Root Ceiling (Optional)"` (`"Set Global Ceiling"` / `"Clear Global Ceiling"`) with honest wording; keep `enforce_agy_sandbox` global only.
- `crates/agemax-app/src/main.rs`:
  - Pure enforcement helpers (`validate_against_roots`, `can_change_workspace_folder`, `resolve_restored_pane_cwd_with_roots`, `validate_ipc_workspace_target`) with unit tests covering restore, folder-change blocking, and independent IPC across workspaces.
  - Wire `ToggleActiveWorkspaceLock`, `ToggleWorkspaceLock(idx)`, `save_session()`, `restore_session_snapshot`, and `create_workspace_with_backend`.
