# Age Max (`agemax`)

**Age Max** is a native Windows terminal workspace and multi-agent mission control written in Rust for running, observing, and coordinating multiple Antigravity CLI (`agy`) sessions and shells side by side.

> **Pure Host, Strict Security Boundary**: Age Max contains **no embedded AI models, LLMs, or webview app shells**, and **never reads, stores, or proxies user credentials or OAuth tokens**. Intelligence and authentication live strictly inside the official CLI tools you launch.

---

## Why Age Max?

Running multiple autonomous CLI agents in parallel turns ordinary terminal tabs into guesswork:

* You lose track of which agent is **thinking**, **running a tool**, **waiting on your approval**, or **finished**.
* Closing a terminal tab on Windows can leave orphaned child processes holding file locks.
* Browser previews, WSL distros, SSH targets, and AI chats live in disconnected windows.

Age Max solves this with a single native Win32/DirectX-backed (`eframe`/`egui` + `ConPTY` + `alacritty_terminal`) workstation designed ground-up for **Windows 10 (1809+) and Windows 11**.

---

## Key Features

### Live Agent Mission Control
* **Granular Agent States**: Tracks `Idle`, `Working`, `Thinking`, `RunningTool`, `WaitingForInput`, `Done`, and `Failed` from **OSC 7501** sequences, terminal bells/OSC 9/99/777, structured **`agy acp`** stdio streams, and PTY activity bursts.
* **Animated State Borders & Procedural Mascots**: Pane borders smoothly ease over ~200 ms between state colors (subtle glow for thinking/tools, amber pulse when awaiting input, transient green flash on completion) while keeping active keyboard focus visually distinct. Clean-room procedural vector mascots reflect session states right inside pane headers and overview cards.
* **Mission-Control Sidebar Cards**: Displays live status icons, active working timer, rolling 16-second zero-allocation output sparkline, Git branch & background-throttled diff churn (`+N -M`), unread badges, and latest terminal output line.
* **All-Agents Grid View (`⊞ Grid`)**: Overview card grid across all workspaces and tabs so you can monitor parallel sessions at a glance and click straight into any pane.
* **Global Activity Feed (`📡 Feed`) & Attention Alerts (`🔔 Alerts`)**: Chronological milestone timeline alongside unread notifications, plus WinRT **Windows Toast notifications**, taskbar flashing (`FlashWindowEx`), and an optional **synthesized WAV attention chime** (with debounce cooldown).
* **Safe Multi-Session Broadcast (`📣 Broadcast`)**: Dispatch a prompt or command to multiple sessions only after reviewing an explicit per-pane checklist showing workspace name, backend (`Local` / `WSL` / `SSH`), and working directory.
* **Welcome & Quick-Launch View (`🏠 Home`)**: Instant access to *Start Agent*, *Safe Plan Mode* (`agy --mode plan`), *Multi-Agent Team Grid*, and recent sessions from `session.json`.

### Workspaces, Backends & Panels
* **Workspaces -> Tabs -> Binary Split Panes**: Deterministic layout tree with draggable split dividers, zoom mode, mouse wheel scrollback, drag selection copy/paste, right-click context menu, `Ctrl+Click` hyperlinks, and live font zoom.
* **Local ConPTY, WSL & Remote SSH**: Native PowerShell 7 / Windows PowerShell / CMD via ConPTY, plus WSL (`wsl.exe`) and OpenSSH (`ssh.exe`) workspace backends.
* **Embedded WebView2 Preview Pane (`🌐 Preview`)**: Scriptable live browser pane controlled directly from the toolbar or CLI (`agemax browser open|screenshot|eval`).
* **Structured `agy acp` Card Panel (`🤖 ACP`) & Team Grid (`👥 Team`)**: Interactive tool/artifact/plan card view over JSON-RPC stdio and one-click Coordinator + Worker split tabs.

### Safety, Privacy & Windows Hygiene
* **Safe Folder Lock (`allowed_root_dir`)**: Restrict workspace creation, session restores, Explorer launches, and IPC commands strictly inside an approved directory tree (normalizing `..` traversal and symlinks).
* **Working Directory Visibility & `agy --sandbox`**: Always shows the active workspace directory and lock state in the top folder bar, with a one-click `enforce_agy_sandbox` toggle (`--sandbox`).
* **Zero Orphaned Child Processes**: Every spawned shell and ACP child process is bound to a Windows Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
* **DACL-Secured Local IPC**: Named pipe `\\.\pipe\agemax-<username>` restricts access exclusively to the current Windows user SID and Local System.
* **Zero Telemetry**: 100% local state (`%APPDATA%\AgeMax\config.toml`, `%LOCALAPPDATA%\AgeMax\session.json`).

### Themes & Desktop Polish
* **6 Built-In Themes**: `Midnight Dark`, `High Contrast`, `Soft Light`, `Cyberpunk`, `Terminal Green`, and `Dracula` with live TOML hot reload.
* **Guided & Minimal Chrome Modes**: Switch between button-first *Guided Mode* (toolbar + pane headers) and distraction-free *Minimal Mode* via Settings or the Command Palette.
* **Debounced Pane Animations & Opt-In Mica**: Smooth 140 ms visual pane open/close transitions with ConPTY resize debouncing, plus an optional Windows 11 **Mica Backdrop** in Settings (falls back transparently to opaque themes on Windows 10).

---

## Architecture

Age Max is organized as a clean-room modular Cargo workspace (`MIT OR Apache-2.0`):

| Crate | Purpose |
| :--- | :--- |
| [`agemax-core`](crates/agemax-core) | Pure domain models: `Workspace`, `Tab`, `LayoutTree`, `AgentStatus`, `NotificationStore`, `ActivityFeed`, `SessionSnapshot` |
| [`agemax-term`](crates/agemax-term) | Windows ConPTY & WSL/SSH backends, Job Objects, 4 ms coalescer, `alacritty_terminal` VT grid, git branch & diff stat helpers |
| [`agemax-agy`](crates/agemax-agy) | Official `agy` CLI adapter, OSC 7501/9/777 parser, live `agy acp` JSON-RPC client, isolated profiles, quota query |
| [`agemax-ipc`](crates/agemax-ipc) | Single-instance activation, Win32 DACL named pipe server/client, JSON-RPC 2.0 protocol |
| [`agemax-config`](crates/agemax-config) | TOML config schema, safety path guard (`validate_workspace_path`), hot-reload watcher, themes, keybindings |
| [`agemax-app`](crates/agemax-app) | Native GUI (`eframe`/`egui`), Mission Control views, procedural mascots, WebView2 pane, DWM Mica, WinRT Toasts |
| [`agemax-cli`](crates/agemax-cli) | Companion CLI (`agemax.exe`) for automating workspaces, panes, attention states, and browser previews over IPC |

---

## Getting Started

### Prerequisites
* **OS**: Windows 10 (1809+, build 17763+) or Windows 11 (`x64`)
* **Toolchain**: [Rust stable toolchain](https://rustup.rs/) (2021 edition) with MSVC build tools
* **Optional**: [Antigravity CLI (`agy`)](https://antigravity.google/cli/install.ps1) and Microsoft Edge WebView2 Runtime (preinstalled on Windows 11)

### Build & Run from Source

```powershell
# Clone the repository (uses develop branch for active development)
git clone https://github.com/hatimbenayad/Agemax.git
cd Agemax

# Run tests and lint checks
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings

# Build optimized release binaries
cargo build --release --workspace

# Launch the GUI multiplexer
.\target\release\agemax-app.exe
```

Release binaries are written to:
* `target/release/agemax-app.exe` — Native Age Max workstation GUI
* `target/release/agemax.exe` — Local IPC automation CLI

Optional Windows Explorer context menu ("Open Folder in Age Max") scripts and Inno Setup / WinGet manifests are located in [`packaging/windows/`](packaging/windows).

---

## Companion CLI (`agemax.exe`)

Control an already-running `agemax-app.exe` window over `\\.\pipe\agemax-<username>`:

```powershell
# Workspaces & layout
agemax workspace list
agemax workspace create --name "Backend" --cwd C:\Projects\api
agemax split --direction right
agemax focus <pane-uuid>

# Terminal automation & agent signaling
agemax send-keys "cargo test`r"
agemax read-screen
agemax status done --message "Tests passed"
agemax notify "Needs plan approval" --status waiting

# Focus running window on a directory (single-instance activation)
agemax activate --cwd C:\Projects\api

# Control embedded WebView2 browser preview panes
agemax browser open http://localhost:3000
agemax browser eval "document.title"
agemax browser screenshot
```

---

## Default Keyboard Shortcuts

All bindings are configurable in **Settings (`⚙`) -> Shortcuts** or `%APPDATA%\AgeMax\config.toml`. Defaults preserve standard terminal control signals (`Ctrl+C`, `Ctrl+D`, `Ctrl+Z`, `Ctrl+K`), with an optional **`Use Ctrl+K Command Palette`** preset in Settings:

| Action | Default Shortcut |
| :--- | :--- |
| Command Palette | `Ctrl+Shift+P` |
| New Tab | `Ctrl+Shift+T` |
| Split Pane Right | `Ctrl+Shift+D` |
| Split Pane Down | `Ctrl+Shift+R` |
| Close Active Pane | `Ctrl+Shift+W` |
| Zoom / Unzoom Pane | `Ctrl+Shift+Enter` |
| Toggle Sidebar | `Ctrl+B` |
| Toggle Alerts & Feed Drawer | `Ctrl+Shift+N` |
| Jump to Oldest Waiting Agent | `Ctrl+Shift+U` |
| Copy / Paste Selection | `Ctrl+Shift+C` / `Ctrl+Shift+V` |
| Directional Pane Focus | `Alt+Arrow Keys` |
| Font Zoom | `Ctrl++` / `Ctrl+-` / `Ctrl+0` / `Ctrl+Wheel` |

---

## Configuration (`config.toml`)

Global settings live in `%APPDATA%\AgeMax\config.toml` (with optional per-project custom commands in `<workspace>\agemax.toml`) and reload automatically on save:

```toml
[general]
font_size = 14.0
ui_mode = "guided"               # "guided" | "minimal"
enable_toast_notifications = true
enable_taskbar_flash = true
enable_sound_notifications = true
show_agent_mascots = true
enable_mica_backdrop = false     # Windows 11 DWM Mica opt-in
enforce_agy_sandbox = false      # Pass --sandbox when launching agy
# allowed_root_dir = "C:\\Users\\you\\Projects"
# default_workspace_dir = "C:\\Users\\you\\Projects"
```

---

## Development & Specifications

Age Max follows clean-room, Spec-Driven Development under [`specs/`](specs):
* [`specs/00-vision.md`](specs/00-vision.md) — Product scope and non-goals
* [`specs/01-requirements.md`](specs/01-requirements.md) — Testable functional & performance requirements (`REQ-001`–`REQ-089`)
* [`specs/03-architecture.md`](specs/03-architecture.md) — Crate boundaries, ConPTY threading, and IPC flows
* [`specs/04-units/`](specs/04-units) — Unit-by-unit implementation specifications

---

## License

Dual-licensed under **`MIT OR Apache-2.0`**. See [`LICENSE`](LICENSE) for details. Dependency licenses are enforced in CI via [`deny.toml`](deny.toml).
