# 02 - User Stories

This document defines user personas, scenarios, and acceptance criteria for **Age Max**.

---

## Personas

1. **Alex (Senior Systems / Backend Engineer)**:
   - Works across three microservices simultaneously.
   - Runs `agy` sessions in each service to generate unit tests, refactor database models, and write migration scripts.
   - Needs clear status visibility across tasks without switching between three bloated IDE windows.

2. **Priya (Full-Stack Engineer & Team Lead)**:
   - Maintains a frontend app and backend API.
   - Deploys multiple `agy` agents in parallel: one exploring UI components, one fixing API endpoints.
   - Uses different Antigravity accounts (work Google account vs personal quota account).
   - Needs prompt notification when an agent asks for plan approval, without getting distracted while context-switching.

3. **Marcus (DevOps / CLI Power User)**:
   - Prefers keyboard-driven navigation, tiling window managers, and terminal multiplexers.
   - Scripts his daily setup via PowerShell and expects an IPC CLI to orchestrate panes, send commands, and query status.

4. **Sam (Non-Technical Creator / Solo Founder)**:
   - Wants AI coding agents (`agy`) to help build websites, automations, and scripts, but is uncomfortable memorizing terminal flags or keyboard chords.
   - Prefers clicking clear buttons (*Start Agent*, *Split Screen*, *Choose Folder*), visual settings, mouse Copy/Paste, and friendly status badges—while still letting power users like Marcus switch to Minimal view.

---

## User Stories

### US-01: Multi-Agent Parallel Execution
- **As** Alex,
- **I want to** run three `agy` agent sessions in separate panes within a single workspace window,
- **So that** I can observe all agents working on different repository modules at the same time without losing terminal context.
- **Acceptance Criteria**:
  - Alex can split a tab horizontally and vertically (`Ctrl+Shift+D`, `Ctrl+Shift+R`).
  - Each pane runs an independent ConPTY process hosting `agy`.
  - Panes can be resized smoothly with mouse drag or keyboard shortcuts (`Alt+Arrow`).
  - One pane can be zoomed to fullscreen (`Ctrl+Shift+Enter`) and un-zoomed.

### US-02: Instant Attention & Notification on Approval
- **As** Priya,
- **I want to** receive an immediate visual cue and Windows toast when an agent requires approval or completes a turn,
- **So that** I can focus on other documents and return immediately when my input is required.
- **Acceptance Criteria**:
  - When `agy` enters waiting state (via OSC 7501 or bell), the pane border turns amber/yellow.
  - The sidebar shows an unread badge with a summary message.
  - A Windows Toast notification appears if Age Max is in the background.
  - Clicking the toast or pressing `Ctrl+Shift+U` jumps focus directly to the waiting pane.
  - Entering input dismisses the notification ring.

### US-03: Seamless Session Persistence and Resume
- **As** Alex,
- **I want to** shut down my machine at the end of the day and reopen Age Max next morning with my workspace layout and agents ready to resume,
- **So that** I don't waste 15 minutes setting up directories, splits, and conversation contexts.
- **Acceptance Criteria**:
  - On exit, Age Max saves the workspace tree, pane splits, working directories, and agent conversation IDs to `%LOCALAPPDATA%\AgeMax\session.json`.
  - On launch, the exact workspace layout and panes are restored.
  - Panes that were running `agy` display a "Resume Agent" button or prompt that executes `agy --conversation <id>`.

### US-04: Account Profile Switching Without Credential Risk
- **As** Priya,
- **I want to** run an agent session using my secondary Antigravity profile in a dedicated workspace,
- **So that** I keep my personal quota and project history isolated from my work account without sharing credentials.
- **Acceptance Criteria**:
  - Priya can configure a profile with a distinct label and directory.
  - Age Max launches `agy` with the profile's environment without touching credentials.
  - First-time login happens directly inside the terminal pane via the official Google login flow.
  - The sidebar displays the active profile tag next to each workspace.

### US-05: Scripting Multiplexer from External Tools
- **As** Marcus,
- **I want to** control Age Max from my terminal using the `agemax` CLI over a local named pipe,
- **So that** I can automate workspace initialization and trigger agents from my build scripts.
- **Acceptance Criteria**:
  - `agemax workspace create --name "Backend" --dir "C:\src\backend"` creates a workspace.
  - `agemax split --direction right --command "agy"` splits the active pane.
  - `agemax read-screen <pane-id>` dumps visible terminal text.
  - `agemax notify --pane-id <id> --status done` triggers an attention notification.
  - All communication uses the Windows Named Pipe protected by the user's logon SID.

### US-06: Clean Child Process Tree Termination
- **As** Alex,
- **I want to** close an agent pane and be confident that all child processes spawned by that agent are completely terminated,
- **So that** compilers, test runners, or dev servers don't remain running as zombies locking project files.
- **Acceptance Criteria**:
  - Closing a pane terminates the ConPTY process and its entire process tree via Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
  - Verifiable via Task Manager / Process Explorer: zero orphaned Node/Python/git processes after pane closure.

### US-07: Choosing Between Guided Buttons and Minimal Terminal Chrome
- **As** Sam (or Marcus),
- **I want to** toggle between a "Guided" UI mode (with top toolbar buttons, per-pane headers, status pills, and shortcut hints) and a "Minimal" UI mode without losing any keyboard shortcuts,
- **So that** newcomers get discoverable mouse controls while terminal minimalists keep a clean, zero-chrome canvas.
- **Acceptance Criteria**:
  - Settings (`config.toml` and UI switch) allow choosing `ui_mode = "guided"` or `"minimal"` plus per-element overrides (`show_toolbar`, `show_pane_headers`).
  - Switching UI mode updates the layout immediately without restarting terminals.
  - All existing keyboard shortcuts remain active and configurable in both modes.

### US-08: Picking a Project Folder Without `cd` Commands
- **As** Sam,
- **I want to** click `+ New Workspace`, browse to my project folder using the standard Windows folder dialog, give it a friendly name, and optionally start `agy` inside it,
- **So that** I don't have to type Windows file paths into a command line.
- **Acceptance Criteria**:
  - `+ New Workspace` opens a creation dialog with a native Windows Folder Picker button.
  - Users can rename workspaces and tabs inline from the UI.
