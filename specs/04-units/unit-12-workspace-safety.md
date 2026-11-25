# Unit 12: Workspace Folder Lock, Path Visibility & Agent Sandbox

## 1. Scope
Give non-technical and security-conscious users unmistakable visibility and strict control over where Age Max opens folders and launches Antigravity (`agy`):
1. Visible **Current Working Folder & Lock Indicator Bar** above panes + prominent folder display in Agent Launcher (`REQ-080`).
2. Configurable **`default_workspace_dir`** and **`allowed_root_dir`** in `config.toml` and visual **🛡 Safety & Folders** tab in `SettingsModal`, enforced across startup, `NewWorkspaceModal`, `session.json` restore, single-instance activation, and `workspace.create` IPC (`REQ-081`).
3. Honest transparency & **`agy --sandbox`** toggle (`enforce_agy_sandbox`) inside Settings and Agent Launcher (`REQ-082`).

## 2. Requirements Traceability
- **REQ-080**, **REQ-081**, **REQ-082**.

## 3. Delivered Modules & Tests
- **Configuration & Path Guard (`REQ-081`)**:
  - `crates/agemax-config/src/schema.rs`: `GeneralConfig.default_workspace_dir`, `allowed_root_dir`, `enforce_agy_sandbox`.
  - `crates/agemax-config/src/safety.rs`: `validate_workspace_path`, `resolve_startup_directory`, `display_clean_path`, `PathSafetyError`.
- **Agent Sandbox Support (`REQ-082`)**:
  - `crates/agemax-agy/src/adapter.rs`: `build_preset_command_with_options`, `shell_line_for_preset_with_options` passing `--sandbox` when enabled.
- **UI Visibility, Settings & Enforcement (`REQ-080`, `REQ-081`, `REQ-082`)**:
  - `crates/agemax-app/src/chrome_ui.rs`: `render_workspace_folder_bar` (`📁 Working in:` + clickable Lock status pill), live path validation inside `NewWorkspaceModal`.
  - `crates/agemax-app/src/onboarding_settings.rs`: Dedicated `🛡 Safety & Folders` tab (`SettingsTab::Safety`), folder pickers for Safe Folder Lock & Default Start Folder, `AgentLauncherModal` working folder banner + `--sandbox` checkbox.
  - `crates/agemax-app/src/main.rs`: Enforced across cold startup, `session.json` restore, `NewWorkspaceModal`, `workspace.create` IPC, and single-instance `app.activate` handoff.
- **Tests**:
  - `crates/agemax-config/tests/safety_tests.rs`
  - `crates/agemax-agy/tests/sandbox_preset_tests.rs`
