# ADR-008: Independent Per-Workspace Folder Locks & Global Ceiling Separation

- **Status**: Accepted
- **Date**: 2026-11-26
- **Related Requirements**: REQ-091, REQ-092, REQ-093, REQ-094, REQ-095

## Context

Previously, `GeneralConfig.allowed_root_dir` acted as a single global folder fence across all workspaces. Users who clicked the folder-bar pill (`"Unrestricted Folder (Click to Lock…)"`) were sent to `Settings > Safety & Folders`, where applying a lock fenced *every* workspace to that single folder tree—making it impossible to lock Workspace A to Folder A while keeping Workspace B unrestricted or locked to Folder B.

Empirical spikes (S6 and S6b) also established that `agy --sandbox` does not restrict `agy`'s file-edit tool (`write_file`) to the workspace folder.

## Decision

1. **Per-Workspace Lock (`locked_root`)**: Add `#[serde(default)] pub locked_root: Option<PathBuf>` to `Workspace` (`agemax-core/src/workspace.rs`) and `WorkspaceSnapshot` (`agemax-core/src/session.rs`), keeping `SessionSnapshot::CURRENT_VERSION = 1`.
2. **One-Click Toggle**: Clicking the folder-bar pill toggles the *active workspace's* `locked_root` (`None` ↔ `Some(ws.cwd.clone())`) directly without opening Settings. The sidebar shows a lock badge on locked workspace cards and provides a right-click context menu toggle. `NewWorkspaceModal` offers an opt-in `"Lock this workspace to its folder"` checkbox (default unchecked).
3. **Global Ceiling Preservation**: Retain `GeneralConfig.allowed_root_dir` as an optional **App-Wide Safe Root Ceiling** in `Settings > Safety & Folders` (relabeled clearly). A local path is valid for a workspace iff it satisfies both `allowed_root_dir` (if set) and `ws.locked_root` (if set).
4. **Global `--sandbox` Only**: Keep `GeneralConfig.enforce_agy_sandbox` global only (no per-workspace override).
5. **Honest Wording**: All lock tooltips, Settings descriptions, and modal help text must state plainly that locking tells Age Max where to open, restore, and accept requests, and does not confine what AI agents or terminal shells can read or write on the filesystem.

## Consequences

- Locking one workspace never restricts any other workspace.
- Existing `config.toml` and `session.json` v1 files remain 100% backward compatible.
- `PreToolUse` hook scaffolding (`.agents/hooks.json`) is explicitly excluded from Unit 14b and deferred to an optional future Unit 14d.
