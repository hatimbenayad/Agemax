# Unit 13: Alive Mission Control & Desktop Polish

## 1. Scope & Multi-Branch Structure
Unit 13 upgrades Age Max into a lively, observant multi-agent mission control across **three focused feature branches**, each merged independently into `develop` (and preserved permanently on GitHub):

1. **Branch `feat/unit-13a-themes-glow-mascots`** (`REQ-083`, `REQ-084`, `REQ-085`, `REQ-086`):
   - **Theme Presets**: `Cyberpunk`, `TerminalGreen`, and `Dracula` added to `ThemePreset` alongside `MidnightDark`, `HighContrast`, and `SoftLight`.
   - **Eased State-Colored Glow & Per-State Animations**: ~200 ms RGB/alpha color interpolation on pane borders + soft state-colored outer glow, transient `Done` green flash, breathing pulse while working, soft amber pulse when waiting, distinct inner keyboard focus accent, and automatic idle repaint suspension (`REQ-061` < 1% CPU).
   - **Procedural Agent Mascots**: Pure clean-room `egui::Painter` vector companion anchored cleanly in pane headers and cards (`show_agent_mascots` toggle in Settings -> Appearance).
   - **Subtle Attention Chime**: Optional debounced sound alert (`enable_sound_notifications`, 3 s cooldown) using clean-room in-memory PCM WAV playback on Windows (`PlaySoundW`).

2. **Branch `feat/unit-13b-mission-control`** (`REQ-087`, `REQ-088`):
   - **Granular `Thinking` vs `RunningTool` States**: Backward-compatible additions to `AgentStatus`, exact mapping from `LiveAcpSession` (`agy acp`) + OSC 7501 (`status=thinking`, `status=tool`) + PTY stream heuristics.
   - **Mission-Control Sidebar Cards**: Last non-empty output snippet, active working elapsed timer, fixed-size zero-alloc bytes/sec activity sparkline, and throttled background git diff stats (`+N -M`).
   - **Global Activity Feed**: Chronological milestone timeline tab inside the right drawer (agent status changes, ACP tools/artifacts, workspace events).
   - **All-Agents Grid View**: Overview grid across all workspaces and panes with mascots, states, sparklines, git stats, last line, and one-click jump to pane.
   - **Safe Broadcast Prompt**: Modal allowing prompt dispatch only to explicitly confirmed/checked active terminal/ACP panes (excluding exited shells and preview panes).
   - **Welcome / Quick-Launch View (`🏠 Home`)**: Clean overlay/empty-state launcher featuring Age Max ASCII emblem, one-click Start Agent cards, and Recent Workspaces without slowing terminal startup (`REQ-058`).

3. **Branch `feat/unit-13c-compositor-polish`** (`REQ-089`):
   - **Debounced Animated Pane Open/Close**: Smooth visual container transitions (~140 ms) that debounce ConPTY resize ioctls and WebView2 HWND repositioning until settled to eliminate shell prompt line-wrap garbling.
   - **Opt-In Windows 11 Mica Backdrop**: Configurable `enable_mica_backdrop` (`DwmSetWindowAttribute` `DWMWA_SYSTEMBACKDROP_TYPE` + transparent framebuffer composition + safe opaque fallback on Windows 10).
   - **Optional `Ctrl+K` Command Palette Preset**: Keeps `Ctrl+Shift+P` default (`REQ-054` terminal `\x0b` protection) while offering an opt-in IDE-style `Ctrl+K` preset in Settings -> Shortcuts.

## 2. Requirements Traceability
- **REQ-083**, **REQ-084**, **REQ-085**, **REQ-086**, **REQ-087**, **REQ-088**, **REQ-089**.
