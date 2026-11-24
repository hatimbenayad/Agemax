# Unit 11: Windows Packaging, Single-Instance IPC Activation & Unit 7 Stretch Hooks

## 1. Scope
1. Embed `.ico` Windows application icon, add Windows installer configuration (Inno Setup / MSI manifest + `winget` metadata), and optional Explorer "Open Folder in Age Max" context menu script.
2. Add single-instance named-pipe detection on `agemax-app.exe` launch (`\\.\pipe\agemax-<username>`): focus existing window and optionally open the passed directory rather than colliding.
3. Follow-up foundation toward [Unit 7 Stretch Features](file:///c:/Users/hatim/Downloads/agemax/specs/04-units/unit-7-stretch-features.md) (`agy acp` visual card panel, WebView2 local preview pane).

## 2. Requirements Traceability
- **REQ-041**–**REQ-044**, **REQ-078**, **REQ-079**.

## 3. Delivered Modules & Tests
- **Icon & Resource Embedding (`REQ-078`)**:
  - `assets/agemax.ico`: Multi-size (16, 32, 48, 256) RGBA Windows `.ico`.
  - `crates/agemax-app/build.rs`: Invokes Windows SDK `rc.exe` to embed `1 ICON` and `VERSIONINFO` resources into `agemax-app.exe`.
  - `crates/agemax-app/src/app_icon.rs`: Generates runtime `64x64` RGBA `IconData` for `ViewportBuilder::with_icon`.
- **Windows Packaging & Explorer Menu (`REQ-078`)**:
  - `packaging/windows/agemax.iss`: Inno Setup 6 installer script (per-user `%LOCALAPPDATA%\Programs\AgeMax`, optional PATH and Explorer menu).
  - `packaging/windows/open-in-agemax-install.ps1` & `open-in-agemax-uninstall.ps1`: HKCU Explorer `"Open Folder in Age Max"` registry helpers.
  - `packaging/windows/winget/hatimbenayad.AgeMax*.yaml`: WinGet v1.6 manifest set.
  - Updated `.github/workflows/ci.yml` portable zip packaging step.
- **Single-Instance Activation (`REQ-079`)**:
  - `crates/agemax-ipc/src/activation.rs`: `LaunchArgs::parse_from` and `try_activate_existing_instance` forwarding `app.activate` over `\\.\pipe\agemax-<username>` with `AllowSetForegroundWindow(ASFW_ANY)`.
  - `crates/agemax-app/src/ipc_bridge.rs` (`IpcAppCommand::Activate`), `crates/agemax-app/src/notification_win.rs` (`activate_window`, `find_current_process_hwnd`), `crates/agemax-cli/src/main.rs` (`agemax activate [--cwd]`), and `crates/agemax-app/src/main.rs`.
- **Unit 7 Stretch Foundation Hooks**:
  - `crates/agemax-core/src/model.rs`: `SurfaceKind` (`Terminal`, `BrowserPreview`, `AgentCardPanel`).
  - `crates/agemax-agy/src/acp.rs` & `AntigravityAdapter::build_acp_command`: NDJSON ACP event parser (`AcpEvent`, `AcpCardState`) for `agy acp --stdio`.
- **Automated Tests**:
  - `crates/agemax-ipc/tests/single_instance_tests.rs`
  - `crates/agemax-core/tests/surface_kind_tests.rs`
  - `crates/agemax-agy/tests/acp_hook_tests.rs`
  - `crates/agemax-app/tests/packaging_tests.rs`
