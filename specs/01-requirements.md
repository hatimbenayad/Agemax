# 01 - Functional & Non-Functional Requirements

This document specifies the testable requirements for **Age Max**. Every requirement is identified by a unique ID (`REQ-xxx`) and must map directly to automated tests and acceptance criteria in the unit specifications.

---

## 1. Terminal Engine & ConPTY Integration

- **REQ-001**: The system MUST spawn Windows pseudo consoles via ConPTY (`CreatePseudoConsole`) on Windows 10 (1809+) and Windows 11.
- **REQ-002**: The system MUST detect the default shell in order of preference: PowerShell 7 (`pwsh.exe`) if present on PATH, otherwise Windows PowerShell (`powershell.exe`). It MUST also support configuring Command Prompt (`cmd.exe`) and WSL (`wsl.exe`).
- **REQ-003**: The terminal emulator MUST support full VT100/VT220/xterm escape sequences, including alternate screen buffer, scroll regions, cursor visibility/shape, bracketed paste mode, and 24-bit truecolor.
- **REQ-004**: The terminal engine MUST support mouse event reporting (SGR mouse mode 1006) for interactive CLI applications.
- **REQ-005**: The terminal engine MUST support OSC 7 (current working directory reporting), OSC 8 (hyperlinks), and OSC 52 (clipboard manipulation).
- **REQ-006**: The terminal engine MUST support terminal resizing with text reflow on the active buffer and scrollback history.
- **REQ-007**: The system MUST support scrollback navigation, text selection, and standard clipboard copy/paste without breaking terminal input modes.
- **REQ-008**: The PTY reader MUST run on a dedicated thread per surface and deliver output into a bounded channel to prevent blocking the UI thread.
- **REQ-009**: PTY output writes MUST be batched and coalesced within a ~4 ms time window before triggering a terminal screen refresh.
- **REQ-010**: All spawned child processes for a terminal surface MUST be assigned to a Windows Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` to ensure process tree cleanup.

---

## 2. Workspace, Layout Tree & Navigation

- **REQ-011**: The UI MUST provide a collapsible vertical sidebar displaying a list of active Workspaces.
- **REQ-012**: Each Workspace in the sidebar MUST display: Workspace Name, Git branch name, current working directory, active listening ports, agent operational status, profile label, and latest notification snippet.
- **REQ-013**: The system MUST throttle background discovery tasks (Git branch detection, listening port enumeration) to run asynchronously without UI stutter.
- **REQ-014**: A Workspace MUST contain one or more Tabs.
- **REQ-015**: Each Tab MUST contain a binary layout tree supporting horizontal and vertical pane splits.
- **REQ-016**: The layout tree MUST maintain split ratios and support dynamic resizing via splitter bars.
- **REQ-017**: The layout tree operations (split, close, resize, swap, zoom) MUST be mathematically deterministic and validated via property-based testing.
- **REQ-018**: The UI MUST support directional pane focus navigation (Focus Left, Focus Right, Focus Up, Focus Down) via configurable keyboard shortcuts.
- **REQ-019**: The UI MUST support single-pane "Zoom" mode, temporarily maximizing a pane to fill the tab area, and toggling back to the split layout.
- **REQ-020**: When a pane's root process exits, the pane MUST automatically close or display an exit banner if configured. If the last pane in a tab closes, the tab closes.

---

## 3. Attention System & Notifications

- **REQ-021**: The system MUST intercept OSC 7501 sequences from the PTY stream to extract real-time agent status: `working`, `waiting`, `done`, and `failed`.
- **REQ-022**: The system MUST support terminal bell sequences (`\x07`), OSC 9, OSC 99, and OSC 777 as secondary attention triggers.
- **REQ-023**: When an agent transitions to `waiting` (requires user input) or `done`/`failed`, the pane MUST display a high-contrast colored ring around its border:
  - Yellow/Amber: Waiting for user input / approval.
  - Green: Task completed successfully.
  - Red: Task failed / error encountered.
  - Blue/Cyan: Actively working.
- **REQ-024**: The sidebar entry for the corresponding workspace MUST display an unread attention badge and update its status icon.
- **REQ-025**: The system MUST flash the Windows taskbar window (`FlashWindowEx`) when an attention event occurs while the window is unfocused.
- **REQ-026**: The system MUST display a native Windows Toast notification (`windows::UI::Notifications`) when an attention event occurs in the background.
- **REQ-027**: Clicking a Windows Toast notification MUST bring Age Max to the foreground and immediately focus the triggering pane.
- **REQ-028**: The UI MUST provide an Attention Panel listing all unread notification items with timestamps and message summaries.
- **REQ-029**: The system MUST provide a single global shortcut (default: `Ctrl+Shift+U` or `Ctrl+Alt+A`) to jump focus directly to the most urgent unread pane.
- **REQ-030**: Focusing an attention pane and providing input MUST dismiss the colored ring and clear the unread badge.

---

## 4. Antigravity Adapter & Profiles

- **REQ-031**: The system MUST provide an `AgentAdapter` abstraction to decouple agent-specific logic from generic terminal handling.
- **REQ-032**: The `agemax-agy` crate MUST locate the official `agy.exe` executable by inspecting `%LOCALAPPDATA%\agy\bin\agy.exe`, `%USERPROFILE%\.gemini\bin\agy.exe`, and PATH.
- **REQ-033**: When launching an `agy` session, the adapter MUST set `AGY_CLI_OSC7501=1`, `AGY_CLI_OSC7501_MESSAGES=brief`, `AGY_CLI_DISABLE_AUTO_UPDATE=true`, and `AGY_CLI_HIDE_LOGO=1`.
- **REQ-034**: Mode A (Default): The adapter MUST launch `agy` in the default user environment with zero credential configuration.
- **REQ-035**: Mode B (Profiles): The system MUST support user-defined Profiles, configuring `AGY_ACCOUNT` and/or setting `USERPROFILE` and `HOME` to `%LOCALAPPDATA%\AgeMax\profiles\<name>\`.
- **REQ-036**: Mode B Profiles MUST support choosing between shared history/plugins (via NTFS directory junctions to `.gemini\antigravity-cli`) and fully isolated storage.
- **REQ-037**: Age Max MUST NEVER read, inspect, parse, persist, or transmit user credentials, OAuth tokens, or keyring secrets.
- **REQ-038**: First-time sign-in for any profile MUST occur interactively inside an official `agy` terminal pane.
- **REQ-039**: The system MUST support querying quota via `agy -p "/usage" --output-format json` in throttled background tasks and display remaining limits in the UI.
- **REQ-040**: The adapter MUST implement conversation ID capture via `SessionStart` lifecycle hooks, `conversation_summaries.db`, and `cache\last_conversations.json`.

---

## 5. Local IPC & CLI Control

- **REQ-041**: The system MUST run a local IPC server listening on a Windows Named Pipe formatted as `\\.\pipe\agemax-<username>`.
- **REQ-042**: The named pipe MUST enforce a Windows DACL permitting access only to the current user SID and Local System, blocking unauthorized local processes.
- **REQ-043**: The IPC protocol MUST use JSON-RPC 2.0 framing over the pipe connection.
- **REQ-044**: The companion CLI binary (`agemax.exe`) MUST connect to the named pipe and expose commands to control the running multiplexer.
- **REQ-045**: The CLI MUST support:
  - `agemax workspace create [--name <name>] [--dir <path>]`
  - `agemax workspace list`
  - `agemax workspace close <id>`
  - `agemax split [--direction right|down] [--command <cmd>]`
  - `agemax focus <pane-id>`
  - `agemax send-keys <pane-id> <text>`
  - `agemax read-screen <pane-id> [--lines <n>]`
  - `agemax notify [--pane-id <id>] [--status working|waiting|done|failed] [--message <msg>]`
  - `agemax status set <status>`
  - `agemax resume [--workspace <id>]`
- **REQ-046**: The CLI MUST return exit code 0 on success with structured output (JSON or plain text) and non-zero on error.
- **REQ-047**: The IPC server MUST reject malformed messages gracefully without crashing or stalling the multiplexer.

---

## 6. Session Persistence & Resumption

- **REQ-048**: The system MUST periodically and upon clean shutdown persist the workspace layout, tabs, panes, working directories, and captured agent conversation IDs to `%LOCALAPPDATA%\AgeMax\session.json`.
- **REQ-049**: On application relaunch, the system MUST reload `session.json`, reconstruct the workspace and pane geometry, and restore working directories.
- **REQ-050**: The system MUST NEVER auto-execute unapproved terminal commands from `session.json` without user configuration.
- **REQ-051**: For panes associated with an Antigravity agent, the UI MUST display a one-click "Resume Agent" action that launches `agy --conversation <conversation_id>` in the saved working directory.

---

## 7. Configuration, Custom Commands & Palette

- **REQ-052**: The system MUST load global configuration from `%APPDATA%\AgeMax\config.toml` and optional per-project overrides from `<workspace>\agemax.toml`.
- **REQ-053**: Configuration MUST support hot reloading when configuration files are modified on disk.
- **REQ-054**: Keybindings MUST default to Windows-friendly combinations (using `Ctrl+Shift` modifiers) and MUST NEVER intercept standard terminal signals such as plain `Ctrl+C` or `Ctrl+V`.
- **REQ-055**: The UI MUST provide a Command Palette (`Ctrl+Shift+P`) with fuzzy search over all actions, panes, workspaces, and custom commands.
- **REQ-056**: Configuration MUST allow users to register Custom Commands that appear in the Command Palette and launch predefined shell tasks or agent presets.
- **REQ-057**: Themes MUST be configurable via TOML, defining background, foreground, ANSI colors (16 colors), accent colors, and notification ring colors.

---

## 8. Non-Functional & Performance Budgets

- **REQ-058**: Cold start latency from process launch to first rendered frame MUST be under 300 ms on standard Windows hardware.
- **REQ-059**: Input latency from keystroke delivery to glyph rasterization MUST be under 16 ms.
- **REQ-060**: The system MUST support at least 10 active panes simultaneously streaming high-volume VT output with zero dropped frames or UI stutter.
- **REQ-061**: CPU utilization of the multiplexer process MUST be under 1.0% when all hosted terminal sessions are idle.
- **REQ-062**: The codebase MUST compile cleanly on the stable Rust toolchain (2021 edition) with zero warnings under `cargo clippy --all-targets --all-features -- -D warnings`.
- **REQ-063**: All external dependencies MUST be checked and approved by `cargo-deny` under permissive licenses (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`).
