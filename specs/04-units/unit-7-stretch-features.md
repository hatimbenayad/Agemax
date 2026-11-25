# Unit 7: Stretch Features (Future Phases)

## 1. Scope
Document architectural hooks and forward compatibility for future expansion beyond the core MVP:
1. In-app WebView2 browser pane with scriptable automation API.
2. Remote SSH and WSL workspaces.
3. Multi-agent "team" layouts that spawn dedicated panes per subagent.
4. Embedded agent panel using the Agent Client Protocol (`agy acp`).

## 2. Architectural Design

### A. WebView2 Browser Pane
- Uses Microsoft WebView2 (`webview2-com` crate) embedded as a surface in the layout tree.
- Exposes a scriptable bidirectional message bridge over named pipe IPC:
  - `agemax browser open <url>`
  - `agemax browser screenshot <pane-id>`
  - `agemax browser eval <pane-id> <js>`

### B. WSL and Remote SSH Workspaces
- `PtySystemBackend` trait implementations:
  - `LocalConPtyBackend` (Windows native default)
  - `WslPtyBackend` (launches `wsl.exe -d <distro> --exec <command>`)
  - `SshPtyBackend` (connects via `ssh.exe` or `libssh2`)

### C. Multi-Agent Team Layouts
- Detects subagents spawned via `agy` swarm or subagent tools.
- Automatically creates a tiled grid or split group for coordinating agents, linking them visually with parent-child ring accents.

### D. Agent Client Protocol (ACP) Panel
- Connects to `agy acp --stdio` JSON-RPC server over stdio.
- Provides a dedicated visual panel showing active agent tools, prompt inputs, and real-time artifact generation alongside the raw terminal grid.

## 3. Delivered Modules & Tests
- **A. Embedded WebView2 Preview Pane**:
  - `crates/agemax-app/src/webview_pane.rs`: `WebViewSurface` (static `WebView2LoaderStatic` link via `webview2-com`), URL navigation toolbar (`◀`, `▶`, `↻`, `Go`, `Open External`, `← Terminal`), child `HWND` visibility/bounds synchronization, `ExecuteScript`, and `CapturePreview` PNG screenshots.
  - IPC & CLI: `browser.open`, `browser.screenshot`, `browser.eval` in `crates/agemax-app/src/ipc_bridge.rs` and `agemax browser open|screenshot|eval` in `crates/agemax-cli/src/main.rs`.
  - Test: `crates/agemax-ipc/tests/browser_ipc_tests.rs`.
- **B. WSL & Remote SSH Backends**:
  - `crates/agemax-core/src/workspace.rs` & `session.rs`: `WorkspaceBackend` (`Local`, `Wsl`, `Ssh`).
  - `crates/agemax-term/src/backend.rs`: `PtySystemBackend`, `LocalConPtyBackend`, `WslPtyBackend`, `SshPtyBackend`, `detect_wsl_distros`, `parse_wsl_list_output`.
  - `crates/agemax-app/src/chrome_ui.rs` & `sidebar.rs`: Backend selector tabs in `NewWorkspaceModal` + backend badges.
  - Tests: `crates/agemax-core/tests/team_and_backend_tests.rs`, `crates/agemax-term/tests/backend_tests.rs`.
- **C. Multi-Agent Team Layouts**:
  - `crates/agemax-core/src/layout.rs` (`LayoutTree::create_team_grid`) & `model.rs` (`TeamRoleMeta`).
  - `crates/agemax-app/src/stretch_panels.rs` (`TeamLauncherModal`, role badges & coordinated team rings).
- **D. Live `agy acp` Card Panel**:
  - `crates/agemax-agy/src/acp.rs`: `LiveAcpSession` (Windows Job Object bound, JSON-RPC 2.0 `initialize` + `session/new` + `session/prompt` + `session/update` parser).
  - `crates/agemax-app/src/stretch_panels.rs`: `render_acp_pane` interactive card view & composer.
  - Tests: `crates/agemax-agy/tests/acp_hook_tests.rs`, `crates/agemax-agy/tests/acp_rpc_tests.rs`.
