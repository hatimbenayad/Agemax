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
