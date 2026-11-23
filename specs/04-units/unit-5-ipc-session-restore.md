# Unit 5: Local Named Pipe IPC, CLI & Session Restore

## 1. Scope
Implement the local automation interface and session persistence: Windows Named Pipe JSON-RPC server with DACL security, companion CLI binary (`agemax.exe`), and session state persistence with one-click agent conversation resumption.

## 2. Requirements Traceability
- **REQ-041**: Windows Named Pipe transport (`\\.\pipe\agemax-<username>`).
- **REQ-042**: DACL restricted to current user SID and SYSTEM.
- **REQ-043**: JSON-RPC 2.0 framing.
- **REQ-044**: `agemax-cli` client implementation.
- **REQ-045**: Complete CLI command set (`workspace`, `split`, `focus`, `send-keys`, `read-screen`, `notify`, `status`, `resume`).
- **REQ-046**: Exit code conventions and structured output.
- **REQ-047**: Resilient IPC server error handling.
- **REQ-048**: Session persistence to `%LOCALAPPDATA%\AgeMax\session.json`.
- **REQ-049**: Layout geometry and directory restoration.
- **REQ-050**: Security barrier: no untrusted command execution.
- **REQ-051**: One-click "Resume Agent" launching `agy --conversation <id>`.

## 3. Interfaces & Data Structures
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub version: u32,
    pub active_workspace_id: Uuid,
    pub workspaces: Vec<WorkspaceSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub id: Uuid,
    pub name: String,
    pub cwd: PathBuf,
    pub profile: Option<String>,
    pub tabs: Vec<TabSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub id: Uuid,
    pub cwd: PathBuf,
    pub agent_conversation_id: Option<String>,
    pub last_command: Option<String>,
}
```

## 4. Acceptance Criteria
1. The named pipe server starts on app launch and rejects connection attempts from different user tokens.
2. Running `agemax split --direction right` from an external terminal immediately splits the active pane in Age Max.
3. `agemax read-screen <pane-id>` returns the visible text of the targeted pane.
4. Exiting Age Max writes `session.json`.
5. Relaunching Age Max reconstructs all workspaces, tabs, splits, and working directories identically.
6. Panes previously running `agy` display a "Resume Agent" button that runs `agy --conversation <conversation_id>`.

## 5. Test Plan
- IPC integration test spawning the server in a test thread and issuing client JSON-RPC commands.
- Security DACL test verifying pipe permissions under Windows security API.
- Snapshot serialization and deserialization property tests.
- End-to-end test verifying session restore across app restarts.
