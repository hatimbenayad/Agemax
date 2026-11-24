# Unit 8: UI Modes (`Guided` vs `Minimal`), Button-First Chrome, Dynamic Keybindings & Folder Picker

## 1. Scope
Make Age Max immediately approachable for non-technical users without taking away the clean distraction-free interface or muscle-memory shortcuts expected by technical users:
1. Windows GUI subsystem (`windows_subsystem = "windows"`) in release builds so double-clicking `agemax-app.exe` does not open a spare console window.
2. Configurable UI mode (`Guided` vs `Minimal`, plus granular visibility overrides) saved in `config.toml` and toggleable live in the UI.
3. Top Utility Toolbar, per-pane mini-headers with plain-English status badges & mouse buttons, and an interactive `? Shortcuts` reference modal in `Guided` mode.
4. Dynamic `KeybindingsConfig` matching (preserving `Ctrl+Shift+R`, `Ctrl+Shift+D`, adding `new_tab: "Ctrl+Shift+T"`).
5. Native Windows Folder Picker (`IFileOpenDialog` via `windows`) for `+ New Workspace`, plus workspace rename/close and tab rename in the GUI.

## 2. Requirements Traceability
- **REQ-064**: Release binary uses Windows GUI subsystem (no stray debug console).
- **REQ-065**: `ui_mode` (`Guided` / `Minimal`) and per-element overrides (`show_toolbar`, `show_pane_headers`, `show_shortcut_hints`) in `GeneralConfig` with atomic `config.toml` persistence.
- **REQ-066**: Guided mode top toolbar and compact per-pane headers with plain-English status & action buttons.
- **REQ-067**: Dynamic keybinding parser & matcher wired to `KeybindingsConfig` (adding `new_tab` while preserving all prior shortcuts).
- **REQ-068**: Native Windows Folder Picker dialog (`FolderPicker` trait) inside New Workspace flow.
- **REQ-069**: Workspace rename, workspace close, and tab rename controls in GUI.

## 3. Interfaces & Data Structures
In `agemax-config`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    #[default]
    Guided,
    Minimal,
}

pub struct GeneralConfig {
    pub font_size: f32,
    pub default_shell: Option<String>,
    pub auto_update_check: bool,
    pub ui_mode: UiMode,
    pub show_toolbar: Option<bool>,
    pub show_pane_headers: Option<bool>,
    pub show_shortcut_hints: Option<bool>,
}
```
Helper methods on `GeneralConfig`:
- `effective_show_toolbar(&self) -> bool`
- `effective_show_pane_headers(&self) -> bool`
- `effective_show_shortcut_hints(&self) -> bool`

In `KeybindingsConfig`:
- Add `pub new_tab: String` (default `"Ctrl+Shift+T"`).
- Add structured key chord parser `KeyChord::parse(&str) -> Option<KeyChord>` and `matches(ctrl, shift, alt, key_name)`.

In `agemax-app`:
- `FolderPicker` trait (`pick_folder(&self, initial_dir: Option<&Path>) -> Option<PathBuf>`) with `WindowsFolderPicker` backed by Win32 COM `IFileOpenDialog` (`FOS_PICKFOLDERS`).
- `NewWorkspaceModal` state (name, path, browse trigger, optional `launch_agy: bool`).

## 4. Acceptance Criteria
1. Double-clicking release `agemax-app.exe` opens only the GUI window (no secondary console window).
2. Existing `config.toml` files without `ui_mode` or `new_tab` deserialize cleanly via `#[serde(default)]`.
3. Switching between `Guided` and `Minimal` mode toggles the top toolbar and per-pane headers immediately and persists to `config.toml`.
4. Pressing `Ctrl+Shift+T` opens a new tab; `Ctrl+Shift+R`, `Ctrl+Shift+D`, etc., follow `config.keybindings`.
5. Clicking `+ New Workspace` lets the user browse for a folder via native Windows dialog, name the workspace, and optionally start `agy` inside it; workspaces can be renamed or closed cleanly (stopping child processes via Job Objects).
6. All workspace tests, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo fmt --all -- --check` pass.

## 5. Test Plan
- Unit tests in `agemax-config` for `UiMode` serialization/defaults, granular `effective_*` resolution, atomic `save_to_path`, and `KeyChord::parse` / matching across Windows key combinations.
- Unit tests in `agemax-core` verifying workspace rename/close invariants.
- Compile and verify Windows GUI subsystem attribute & folder picker wrapper.
