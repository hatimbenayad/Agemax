# Age Max UI Upgrade Plan (`docs/ui-upgrade-plan.md`)

**Target**: Transform the existing Age Max UI (`crates/agemax-app`) to match the upgraded dark-purple Mission Control visual target while preserving 100% of existing multiplexer, agent, IPC, and security capabilities.
**Methodology**: AI-DLC + SDD (aligned with [`AGENTS.md`](../AGENTS.md) and [`specs/`](../specs)).
**Constraint**: This document is the **plan only** — no source, config, asset, or dependency changes have been applied.

---

## 1. Summary: Current Architecture & Baseline

### Architecture Overview
Age Max is a native Windows (`10 1809+` / `11`) Rust workspace organized across seven single-responsibility crates:
* `agemax-core` (`crates/agemax-core`): Pure domain state — `Workspace`, `Tab` ([`workspace.rs`](../crates/agemax-core/src/workspace.rs)), binary `LayoutTree` ([`layout.rs`](../crates/agemax-core/src/layout.rs)), `AgentStatus` ([`status.rs`](../crates/agemax-core/src/status.rs)), `NotificationStore`/`ActivityFeed` ([`notification.rs`](../crates/agemax-core/src/notification.rs)), and `SessionSnapshot` ([`session.rs`](../crates/agemax-core/src/session.rs)).
* `agemax-term` (`crates/agemax-term`): Windows ConPTY / WSL / SSH backends ([`backend.rs`](../crates/agemax-term/src/backend.rs), [`pty.rs`](../crates/agemax-term/src/pty.rs)), Win32 Job Object containment (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), 4 ms output batching (`PtyCoalescer`), `alacritty_terminal` (`0.24`) VT grid (`TerminalGrid` in [`vt.rs`](../crates/agemax-term/src/vt.rs)), and background-safe `detect_git_branch` / `detect_git_diff_stats` ([`metadata.rs`](../crates/agemax-term/src/metadata.rs)).
* `agemax-agy` (`crates/agemax-agy`): `agy` CLI launcher presets & `--sandbox`, `OscParser` (OSC 7501/9/777 & Bell), isolated profiles, and `LiveAcpSession` (`agy acp` stdio JSON-RPC).
* `agemax-ipc` (`crates/agemax-ipc`) + `agemax-cli` (`crates/agemax-cli`): Single-instance activation and Win32 DACL-secured local named pipe (`\\.\pipe\agemax-<username>`).
* `agemax-config` (`crates/agemax-config`): TOML config (`%APPDATA%\AgeMax\config.toml`, per-project `agemax.toml`), safe folder confinement `validate_workspace_path` ([`safety.rs`](../crates/agemax-config/src/safety.rs)), hot reload (`ConfigWatcher`), `KeybindingsConfig`, and `ThemeConfig`.
* `agemax-app` (`crates/agemax-app`): `eframe 0.37` + `egui 0.37` (`glow` renderer) desktop app combining [`main.rs`](../crates/agemax-app/src/main.rs) (`AgeMaxApp::logic` & `AgeMaxApp::ui`), [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs), [`sidebar.rs`](../crates/agemax-app/src/sidebar.rs), [`tab_bar.rs`](../crates/agemax-app/src/tab_bar.rs), [`mission_control.rs`](../crates/agemax-app/src/mission_control.rs), [`visual_alive.rs`](../crates/agemax-app/src/visual_alive.rs), [`telemetry.rs`](../crates/agemax-app/src/telemetry.rs), [`webview_pane.rs`](../crates/agemax-app/src/webview_pane.rs), [`stretch_panels.rs`](../crates/agemax-app/src/stretch_panels.rs), [`command_palette.rs`](../crates/agemax-app/src/command_palette.rs), [`onboarding_settings.rs`](../crates/agemax-app/src/onboarding_settings.rs), [`notification_win.rs`](../crates/agemax-app/src/notification_win.rs), and [`backdrop_win.rs`](../crates/agemax-app/src/backdrop_win.rs).

```text
+-----------------------------------------------------------------------------+
| agemax-app (eframe 0.37 / egui 0.37 / glow)                                 |
|   App::logic()                                                              |
|     +- ConfigWatcher::try_recv() -> AgeMaxConfig hot reload                 |
|     +- ipc_rx -> named-pipe JSON-RPC commands                               |
|     +- GitStatsPoller -> background thread -> cached GitDiffStat per ws     |
|     +- LiveAcpSession::poll_events() -> ActivityFeed + AgentStatus          |
|     +- TerminalSurface::process_output_with() -> PTY 4ms -> OscParser + VT  |
|   App::ui()  [CURRENTLY root full_frame -> ui.horizontal -> ui.vertical]    |
|     +- render_sidebar -> ui.vertical (toolbar, folder bar, tab bar, canvas) |
|     +- LayoutTree::compute_rects(content_rect) -> PaneRectAnimator          |
|     +- WebViewSurface (Win32 child HWND) | LiveAcpSession | TerminalSurface |
|     +- render_right_drawer (rendered AFTER ui.vertical closes!)             |
+-----------------------------------------------------------------------------+
```

### Baseline Check Status (Verified on `develop` at `d1e1290`)
* `cargo check --workspace --all-targets`: **PASS** (0 warnings, 0 errors).
* `cargo test --workspace`: **PASS** (all unit, integration, and doc tests across all 7 workspace crates passed, 0 failed, 0 ignored).
* `cargo clippy --all-targets --all-features -- -D warnings`: **PASS** (0 warnings).
* `cargo check --release --workspace`: **PASS** (clean release check).
* `cargo-deny`: Not installed locally (`no such command: deny`); enforced in GitHub Actions (`.github/workflows/ci.yml`), which passed on PR #8 (`d1e1290`).
* Git tags: none currently exist (`git tag -l` empty).

---

## 2. Findings: Root Cause of the 3 Reported Problems

### Problem 1: Missing Icons Rendering as Tofu Boxes (`▯`)
* **Root cause in code**:
  1. `Cargo.toml` configures `eframe = { version = "0.37", default-features = false, features = ["glow", "default_fonts"] }`. In `epaint_default_fonts 0.37.0` (`src/lib.rs`), `default_fonts` bundles ONLY `HACK_REGULAR`, `UBUNTU_LIGHT`, and `EGUI_ICONS` (a 3.5 kB subset holding only Android, Apple, GitHub, Windows, and `git`). `NOTO_EMOJI_REGULAR` and `EMOJI_ICON` are feature-gated behind `monochrome_emoji_fonts`, which is off.
  2. `configure_fonts` ([`crates/agemax-app/src/main.rs:122-151`](../crates/agemax-app/src/main.rs#L122-L151)) loads one monospace system font (`CascadiaCode.ttf` -> fallback `consola.ttf`) **only** into `FontFamily::Monospace`, and loads **zero icon fonts, SVGs, or symbol fallbacks** into `FontFamily::Proportional` (which `egui::Button` and `egui::Label` use).
  3. Over 110 UI strings use Unicode emojis and miscellaneous symbols missing from `UBUNTU_LIGHT`:
     * [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs) (38 sites): `☰`, `⬌`, `⬍`, `▶`, `🌐`, `🤖`, `👥`, `⊞`, `📣`, `🏠`, `⚙`, `🔔`, `⌘`, `🗗`, `⛶`, `📁`, `🔒`, `🔓`, `🛡`, `⟳`, `◐`
     * [`onboarding_settings.rs`](../crates/agemax-app/src/onboarding_settings.rs) (17 sites), [`mission_control.rs`](../crates/agemax-app/src/mission_control.rs) (16 sites), [`stretch_panels.rs`](../crates/agemax-app/src/stretch_panels.rs) (11 sites), [`sidebar.rs`](../crates/agemax-app/src/sidebar.rs) (8 sites: `⎇`, `⚠`, `✎`), [`webview_pane.rs`](../crates/agemax-app/src/webview_pane.rs) (7 sites), [`status.rs`](../crates/agemax-core/src/status.rs) (7 icons: `○`, `⚡`, `◐`, `⚙`, `⚠`, `✓`, `✕`), [`mouse_ux.rs`](../crates/agemax-app/src/mouse_ux.rs) (4 sites), [`tab_bar.rs`](../crates/agemax-app/src/tab_bar.rs) (3 sites), [`command_palette.rs`](../crates/agemax-app/src/command_palette.rs) (`🔍`), [`main.rs`](../crates/agemax-app/src/main.rs).

### Problem 2: Terminal Sits in a Small Fixed-Height Box (~70% Empty Window) and Fails to Fill/Resize
* **Root cause in code**:
  1. `AgeMaxApp::ui` ([`crates/agemax-app/src/main.rs:2153-2256`](../crates/agemax-app/src/main.rs#L2153-L2256)) does **not** use `egui::Panel::left` / `Panel::top` / `Panel::bottom` / `CentralPanel::default()`. As documented in `eframe 0.37.0`'s `App::ui` docstring (`epi.rs:177`), the root `ui` passed to `App::ui` is unstyled and unconstrained. Wrapping everything inside `full_frame.show(ui, |ui| { ui.horizontal(|ui| { ... }) })` triggers three cascading `egui` layout bugs:
  2. `ui.horizontal` (`egui-0.37.0/src/ui.rs:2496-2514`) initializes child region height (`initial_size.y`) to `self.spacing().interact_size.y` (**~18 px**, NOT the OS window height).
  3. Inside that horizontal strip, `render_sidebar` ([`sidebar.rs:42`](../crates/agemax-app/src/sidebar.rs#L42)) calls `ui.allocate_ui(Vec2::new(width, ui.available_height()), ...)`, receiving ~18 px initially and expanding vertically only to fit its internal sidebar stack (~220–240 px for 1 workspace card — roughly 30% of a 720 px window).
  4. Next, `ui.vertical` ([`main.rs:2256`](../crates/agemax-app/src/main.rs#L2256)) runs inside the same height-constrained `ui.horizontal`, renders the toolbar (~60 px wrapped), folder bar (~28 px), and tab bar (~32 px), and then queries `let content_size = ui.available_size();` ([`main.rs:2449`](../crates/agemax-app/src/main.rs#L2449)). In `egui-0.37.0/src/layout.rs:419-443`, `available_from_cursor_max_rect` subtracts `cursor.min.y` from the small row `max_rect.max.y`, yielding a tiny height (clamped by `.max(40.0)` / `.max(32.0)` on lines 2894–2896 or capped near sidebar height) regardless of window size.
  5. Additionally, terminal panes ([`main.rs:2807-3177`](../crates/agemax-app/src/main.rs#L2807-L3177)) paint directly onto `ui.painter()` without calling `ui.allocate_rect(...)` or `ui.expand_to_include_rect(...)`, so the parent `Frame`/`vertical` shrinks to child widget height rather than filling the viewport.
  6. Meanwhile `render_right_drawer` ([`main.rs:3311`](../crates/agemax-app/src/main.rs#L3311), `272 px` wide) is drawn *after* `ui.vertical` has already consumed `available_size().x`, risking horizontal clipping/overlap when opened.
* **Does PTY resize work today?**
  * `TerminalSurface::resize` ([`surface.rs:54-61`](../crates/agemax-term/src/surface.rs#L54-L61)) -> `PtyInstance::resize` ([`pty.rs:287-296`](../crates/agemax-term/src/pty.rs#L287-L296)) **does** resize `TerminalGrid` and call `master.resize(PtySize { rows, cols, ... })`, and [`main.rs:2956-2960`](../crates/agemax-app/src/main.rs#L2956-L2960) invokes it (gated by `PaneRectAnimator::allow_backend_resize` after a 120 ms debounce in [`visual_alive.rs:550-655`](../crates/agemax-app/src/visual_alive.rs#L550-L655)). Because `content_size` from `main.rs:2449` never grows to fill the window, `cols`/`rows` stays stuck in that small box and never tracks window resizing properly.

### Problem 3: 13+ Wrapped Toolbar Buttons & Weak Hierarchy
* **Root cause in code**:
  * `render_top_toolbar` ([`chrome_ui.rs:548-705`](../crates/agemax-app/src/chrome_ui.rs#L548-L705)) places ~13 left buttons + 4 right-to-left buttons inside `ui.horizontal_wrapped`. At `1150 px` window width (`~930 px` content width), text labels force wrapping onto two rows.
  * Every button uses the same default `ui.button` chrome without grouped pill dividers, making primary (`Start agy`), layout (`Split Right`/`Down`/`Grid`), tool (`Preview`/`ACP`/`Team`/`Broadcast`), and utility (`Alerts`/`Actions`/`View`) controls visually flat. `Shortcuts` is embedded inside `SettingsModal` rather than directly reachable from the sidebar footer as in the mockup.

---

## 3. Mockup-to-Code Gap Analysis

| Element (Visual Target) | Exists Today? | What Needs to Change | Where in Code | Difficulty | Risk |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Top Title Bar + Centered Search Pill (`Ctrl+K`) + `— □ ×`** | Partially (`CommandPalette` exists on `Ctrl+Shift+P`; window uses OS title bar) | Add top search bar pill calling `command_palette.open()`, with Workspace entries added to `CommandPalette`. **Framework note**: borderless custom OS title bar (`with_decorations(false)`) requires manual Win32 drag (`ViewportCommand::StartDrag`) & 8-edge resize handles and risks Aero Snap/Mica quirks; recommend keeping native Win32 dark title bar (`DWMWA_USE_IMMERSIVE_DARK_MODE`) or opt-in borderless later. | [`main.rs`](../crates/agemax-app/src/main.rs), [`command_palette.rs`](../crates/agemax-app/src/command_palette.rs), [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs), [`backdrop_win.rs`](../crates/agemax-app/src/backdrop_win.rs) | Medium | Medium (Win32 drag/Snap if borderless) |
| **Collapsible Left Workspace Sidebar (full height)** | Partially | Move from `ui.horizontal` to `egui::Panel::left` so it spans full viewport height cleanly; add subtle resize handle/drag or smooth width collapse, pin `+ New Workspace` below cards, pin `Settings` + `Shortcuts` side-by-side at footer. | [`sidebar.rs`](../crates/agemax-app/src/sidebar.rs), [`main.rs`](../crates/agemax-app/src/main.rs) | Easy | Low |
| **Workspace Cards (dot, name, path, git branch chip, sparkline)** | Partially (`SidebarWorkspaceMeta` already holds branch & `[u32; 16]` sparkline!) | Restyle card to match mockup: colored status dot, name, full/clean path subtitle, pill badge git branch (`main`/`dev`), smooth glowing polyline sparkline (option over bar chart), purple outline on selected card. **Note**: Periodic `detect_git_branch` refresh (not only creation-time) should run inside existing `GitStatsPoller` thread. | [`sidebar.rs`](../crates/agemax-app/src/sidebar.rs), [`telemetry.rs`](../crates/agemax-app/src/telemetry.rs) | Easy | Low |
| **Grouped Single-Row Icon Toolbar + Primary `▶ Start agy` + Overflow (`···`)** | Partially | Replace `ui.horizontal_wrapped` with single-row `Panel::top`; use crisp embedded vector/font icons instead of text+emoji; render primary purple filled pill button for `Start agy`; add vertical separators between groups; collapse secondary buttons into `···` overflow menu when `< 1040 px` (works down to `900 px`). Preserve existing buttons (`Preview`, `ACP`, `Team`, `Broadcast`, `Grid`, `Alerts`, `Actions`). | [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs) | Medium | Medium (hover hints/actions regression if IDs drop) |
| **Alerts Bell Badge (`🔔 2`)** | Partially (`total_unread` exists in `main.rs`) | Paint icon button + circular red count badge overlay on top-right when `total_unread > 0`. | [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs) | Easy | Low |
| **`View: Minimal \| Full` Segmented Toggle** | Partially (today reads `View: Guided` / `Minimal` single button) | Render segmented 2-part pill (`Minimal` \| `Full` where `Full` maps internally to `UiMode::Guided` — zero breaking config migration!). | [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs), [`onboarding_settings.rs`](../crates/agemax-app/src/onboarding_settings.rs) | Easy | Low |
| **Path Bar + Green `🔒 Locked inside folder` Pill** | Yes (`render_workspace_folder_bar`) | Replace broken folder/lock emojis with crisp vector icons; render emerald capsule pill badge matching mockup while preserving click-to-open `SettingsTab::Safety` and `--sandbox` state display. | [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs) | Easy | Low |
| **Rounded Tab Strip with Prompt Glyph & Status Dots** | Partially (`render_tab_bar`) | Upgrade tab pills with terminal `>_` icon, per-tab live status dot (`Waiting`/`Working`/`Done`/`Failed`), close `×`, and `+` button styled cleanly in top strip. | [`tab_bar.rs`](../crates/agemax-app/src/tab_bar.rs) | Easy | Low |
| **Full-Space 2×2 Split Terminal Canvas & Draggable Splitters** | Broken layout sizing today (layout tree & splitter logic already work in `LayoutTree`) | Replace `ui.horizontal`/`ui.vertical` hack with `egui::CentralPanel::default()`; allocate full `available_rect_before_wrap()` via `ui.allocate_rect(...)` so terminal canvas claims 100% remaining width & height at every resolution and OS resize; render crisp visible divider hairline (`2–4 px` hit zone). | [`main.rs`](../crates/agemax-app/src/main.rs) | Medium | **High** (core input/scroll/selection/WebView2 coordinates) |
| **Per-Pane Header (agent label, status pill, maximize/split/close)** | Partially (`render_pane_header` exists) | Show terminal glyph + customizable/auto agent label (`agy-1`, `agy-2` or team role) + rounded pill status badge (`Running`, `Needs input`, `Done`, `Error`, `Ready`) + optional mascot toggle + crisp icon buttons for Zoom/Maximize, Split Right/Down, Close; purple outline on focused pane. | [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs), [`visual_alive.rs`](../crates/agemax-app/src/visual_alive.rs), [`workspace.rs`](../crates/agemax-core/src/workspace.rs) | Easy | Low |
| **Bottom Status Bar (branch, folder, running agents count, CPU/RAM, `UTF-8`, font px, shortcut hint)** | **No** (bar doesn't exist; requires new **CPU/RAM** backend metric) | Add `egui::Panel::bottom` bar (~26 px): left branch dropdown/label, cwd short name, live count (`N agents running` from `pane_status`); right CPU/RAM mini progress pills (**NEW backend**:Win32 `GlobalMemoryStatusEx` + `GetSystemTimes` on 2s throttled worker — zero new crates, OR optional `sysinfo` crate), `UTF-8`, live `{font_size}px`, `{command_palette}` hint. | New [`status_bar.rs`](../crates/agemax-app/src/status_bar.rs), [`telemetry.rs`](../crates/agemax-app/src/telemetry.rs), [`Cargo.toml`](../Cargo.toml), [`main.rs`](../crates/agemax-app/src/main.rs) | Medium | Low (isolated in panel + throttled worker) |

### New Backend Data Assessment
* **Zero new crates needed** for: active agent count (`pane_status`), alerts count (`notification_store`), sparkline (`PaneTelemetry`), git diff churn (`GitStatsPoller`), and live git branch refresh (can refresh `Workspace.git_branch` alongside `GitStatsPoller` on its existing 4s background thread).
* **Only genuinely new OS telemetry**: System **CPU % & RAM %** for the bottom status bar. Using `windows` (`Win32_System_SystemInformation::GlobalMemoryStatusEx` and `Win32_System_Threading::GetSystemTimes`) inside a 2-second throttled background sampler adds **0 external dependencies** and avoids `cargo-deny` surprises. (If user prefers cross-platform compatibility later, `sysinfo` under MIT is the clean alternative — listed in Open Questions).

---

## 4. Proposed Design System

Define visual design tokens in a single source of truth — extending [`crates/agemax-config/src/theme.rs`](../crates/agemax-config/src/theme.rs) + creating a unified UI styling module `crates/agemax-app/src/ui_tokens.rs`:

* **Color Tokens** (Default Dark Purple Mission Control matching the visual target):
  * `bg_canvas`: `#0D0E14` (deep obsidian terminal canvas)
  * `bg_chrome`: `#13141E` (sidebar, title strip, bottom status bar)
  * `bg_surface`: `#191B28` (cards, inactive tabs, inputs, search pill)
  * `bg_elevated`: `#222436` (hovered controls, active tab, selected card fill)
  * `border_subtle`: `#26283B` (hairlines, inactive pane stroke, divider)
  * `border_focus`: `#8B5CF6` (focused pane outline & active workspace card stroke)
  * `accent_primary`: `#7C3AED` / `#8B5CF6` (`Start agy` CTA fill, `Full` segmented active pill, CPU/RAM bar fill)
  * `status_running`: `#22C55E` / `#10B981` (text `#4ADE80`, pill bg `#0E291D`)
  * `status_waiting`: `#F59E0B` (text `#FBBF24`, pill bg `#2E220D`)
  * `status_done`: `#94A3B8` (muted slate/neutral for completed pill or `#34D399` emerald flash)
  * `status_error`: `#EF4444` (alert badge `#EF4444`, error pill)
  * `lock_emerald`: fill `#0E2B22`, border `#1F6E4E`, fg `#34D399`
  * *(Preserve existing 5 additional presets — `HighContrast`, `SoftLight`, `Cyberpunk`, `TerminalGreen`, `Dracula` — deriving matching surface/border tokens cleanly so themes never break.)*
* **Spacing & Radii**:
  * Window outer chrome margin: `0 px` (panels dock flush edge-to-edge)
  * Gutter between split panes: `4 px` (6 px rounded corners on pane frame, 1.5–2 px stroke on focused pane)
  * Control corner radii: `6 px` buttons/tabs, `8 px` workspace cards & search pill, `999 px` (`full`) status/lock capsules
  * Panel dimensions: Top chrome `38–42 px`, Toolbar `42 px`, Path bar `32 px`, Tab strip `34 px`, Pane header `28 px`, Bottom bar `26 px`, Sidebar expanded `228 px` (resizable `180–340 px`) / collapsed `52 px`
* **Typography & Embedded Icons**:
  * Monospace (`FontFamily::Monospace`): `Cascadia Code` -> `Consolas` -> bundled `Hack-Regular` (`13.0 px` default, live zoom `9–32 px`).
  * Proportional UI (`FontFamily::Proportional`): `Segoe UI` (`C:\Windows\Fonts\segoeui.ttf`) if present -> bundled `Ubuntu-Light` (`12.5 px` body, `11.0 px` metadata/caps).
  * **Icon System (`crates/agemax-app/src/icons.rs`)**: Replace all raw Unicode emojis with a dedicated enum `UiIcon` rendered via a single clean strategy: **MIT-licensed Lucide monochrome icon font (`lucide.ttf` embedded via `include_bytes!`, registered at priority in an `Icon` font family) OR deterministic pure `egui::Painter` geometric vector strokes for crisp pixel alignment at 100%/125%/150% DPI**! (Using an embedded MIT icon TTF *or* crisp `Painter` stroke helpers has zero runtime file dependencies, passes `cargo-deny`, and eliminates missing glyphs forever.)

---

## 5. Phased Implementation Plan

Every phase keeps all existing behavior intact, compiles cleanly, and ends with local checks + full smoke test.

### Phase 0: Safety Net & Visual Baseline
* **Goal**: Establish clean rollback points, branch discipline, and baseline screenshots before editing any UI code.
* **Files touched**: None (git + local baseline logs only; spec sign-off).
* **Dependencies**: None.
* **Tasks**:
  1. Create one dedicated branch (`feat/ui-upgrade`) from `develop` (`git tag baseline-pre-ui-upgrade`).
  2. Capture reference baseline screenshots at `1148x745`, `1600x1000`, and `1920x1080` showing current missing glyphs and terminal sizing.
  3. Record exact smoke-test pass baseline for all existing features.
* **Acceptance criteria**: Branch & baseline tag created; baseline tests & screenshots saved outside tracked source (`target/` or artifacts dir).
* **Rollback**: `git checkout develop` (zero code touched).

### Phase 1: Embedded Icon System (Zero Missing-Glyph Boxes)
* **Goal**: Replace every `▯` missing Unicode/emoji character across the entire GUI cleanly across 100/125/150% DPI.
* **Files touched**: `crates/agemax-app/src/icons.rs` (new), `assets/fonts/` (if embedding MIT Lucide TTF, or pure vector in `icons.rs`), `chrome_ui.rs`, `sidebar.rs`, `tab_bar.rs`, `mission_control.rs`, `command_palette.rs`, `onboarding_settings.rs`, `stretch_panels.rs`, `mouse_ux.rs`, `webview_pane.rs`, `main.rs`, `crates/agemax-core/src/status.rs`.
* **New dependencies**: Zero crates needed (`include_bytes!` for MIT Lucide TTF or `egui::Painter` vector paths — note license asset check in `LICENSE`/docs).
* **Tasks**:
  1. Create `icons.rs` defining `UiIcon` variants (`Menu`, `Plus`, `SplitVertical`, `SplitHorizontal`, `Grid`, `Play`, `Eye`, `Cpu`, `Users`, `Broadcast`, `Home`, `Bell`, `Zap`, `Folder`, `Lock`, `Unlock`, `Shield`, `Settings`, `Keyboard`, `Maximize`, `Restore`, `Close`, `GitBranch`, `Search`, `Terminal`, `Alert`, `Check`, `Error`, etc.) + `paint_icon(ui, rect, icon, color)`.
  2. Load `Segoe UI` (`C:\Windows\Fonts\segoeui.ttf`) into `FontFamily::Proportional` fallback chain cleanly in `configure_fonts` alongside `CascadiaCode.ttf` in `Monospace`.
  3. Replace every raw emoji/symbol call site across `chrome_ui.rs`, `sidebar.rs`, `tab_bar.rs`, `mission_control.rs`, `command_palette.rs`, `onboarding_settings.rs`, `stretch_panels.rs`, `mouse_ux.rs`, `webview_pane.rs`, and `status.rs`.
* **Acceptance criteria**: Zero `▯` glyphs visible anywhere in main window, modals, context menu, or Command Palette at 100%/125%/150% DPI.
* **Rollback**: `git revert <phase-1-commits>`.
* **Regression check**: Verify every toolbar button, sidebar button, modal button, and context menu item still triggers its exact action.

### Phase 2: Full-Viewport Panel Skeleton & Dynamic ConPTY Resize
* **Goal**: Fix the ~70% empty space bug permanently by switching root chrome from `ui.horizontal` to `egui::Panel` + `CentralPanel` and making ConPTY resize accurately fill 100% of available space.
* **Files touched**: [`crates/agemax-app/src/main.rs`](../crates/agemax-app/src/main.rs), [`crates/agemax-app/src/sidebar.rs`](../crates/agemax-app/src/sidebar.rs).
* **Dependencies**: None.
* **Tasks**:
  1. Tag `pre-phase-2`. Refactor `AgeMaxApp::ui` panel order:
     - `egui::Panel::top("top_chrome")` (title search bar, toolbar, path bar, tab bar, optional onboarding/restore banners)
     - `egui::Panel::bottom("status_bar")` (placeholder height reservation for Phase 3)
     - `egui::Panel::left("workspace_sidebar")` (collapsible, full vertical height)
     - `egui::Panel::right("right_drawer")` (conditional on `show_notifications_panel` BEFORE `CentralPanel` runs!)
     - `egui::CentralPanel::default()` -> `let content_rect = ui.available_rect_before_wrap(); ui.allocate_rect(content_rect, egui::Sense::hover());`
  2. Fix inner terminal cell loop (`main.rs:2893-2964`) so `cols` and `rows` derive from full `term_rect` / `target_egui_rect`, terminal paints cleanly inside allocated bounds, and `surface.resize(TerminalSize::new(cols, rows))` fires deterministically once `PaneRectAnimator` settles (or immediately on OS window resize / split drag).
  3. Ensure `show_welcome_view` and `show_grid_view` render cleanly inside `CentralPanel` and WebView2 child HWND coordinates still map accurately (`ctx.pixels_per_point()`).
* **Acceptance criteria**: Launching at `1148x745`, maximizing to `1600x1000` / `1920x1080`, or resizing down fills 100% of the central canvas with live terminal rows/cols; `echo $Host.UI.RawUI.WindowSize` in PowerShell reflects actual viewport rows/columns cleanly; Alerts drawer no longer collides with terminal panes.
* **Rollback**: `git revert` Phase 2 commits or `git reset --hard pre-phase-2`.
* **Regression check**: Single pane, split panes, WebView2 bounds, ACP card scroll, Grid view, Welcome view, and folder lock bar all render inside non-overlapping bounds.

### Phase 3: Design System Tokens, Grouped Single-Row Toolbar, Path Bar & Status Bar
* **Goal**: Establish cohesive dark-purple tokens (`ui_tokens.rs`), single-row grouped icon toolbar with overflow menu (`>= 900 px`), styled Path/Lock bar, Top Search Pill, and Bottom Status Bar.
* **Files touched**: `crates/agemax-app/src/ui_tokens.rs` (new), `crates/agemax-app/src/status_bar.rs` (new), [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs), [`main.rs`](../crates/agemax-app/src/main.rs), [`crates/agemax-config/src/theme.rs`](../crates/agemax-config/src/theme.rs).
* **Dependencies**: None.
* **Tasks**:
  1. Extract centralized color/radius/spacing tokens (`UiTokens::from_theme`) supporting `MidnightDark` (tuned closer to deep obsidian/purple target or adding/updating default palette) while preserving all 6 presets cleanly.
  2. Add top header bar with App logo badge (`Age Max`) + centered clickable Search Pill (`Search workspaces, run action... Ctrl+Shift+P` or `Ctrl+K`).
  3. Rebuild `render_top_toolbar` as single-row `ui.horizontal` (no `horizontal_wrapped`) grouping:
     - Layout group (`Sidebar`, `New Workspace/Tab`, `Split Right`, `Split Down`, `Grid View`)
     - Filled purple CTA button (`Start agy`)
     - Tools group (`Preview`, `ACP`, `Team`, `Broadcast`, collapsing into `···` Overflow Menu when width < `1040 px`)
     - Right utility cluster (`Alerts` bell + numeric badge, `Quick Actions/Home`, `View: Minimal | Full` segmented toggle)
  4. Restyle `render_workspace_folder_bar` with emerald `"Locked inside folder"` pill (still clickable -> Safety settings).
  5. Build `render_status_bar` (`Panel::bottom`): git branch chip, folder name, `"N agents running"`, `UTF-8`, `{font_size}px`, shortcut hint (leaving CPU/RAM placeholders ready for Phase 7 or subtle dashed wait).
* **Acceptance criteria**: Toolbar never wraps onto 2 rows down to `900 px` width (overflow menu catches tight widths cleanly); primary `Start agy` stands out clearly; path lock pill & status bar remain crisp at 100/125/150% DPI.
* **Rollback**: `git revert` Phase 3 commits.
* **Regression check**: Every toolbar & overflow action opens its exact modal/view; Minimal mode still hides toolbar/headers cleanly while keeping shortcuts active.

### Phase 4: Sidebar Workspace Cards & Footer
* **Goal**: Upgrade [`sidebar.rs`](../crates/agemax-app/src/sidebar.rs) to match the visual target's workspace status cards, branch chip, smooth polyline sparkline, and pinned Settings/Shortcuts footer.
* **Files touched**: [`sidebar.rs`](../crates/agemax-app/src/sidebar.rs), [`telemetry.rs`](../crates/agemax-app/src/telemetry.rs), [`main.rs`](../crates/agemax-app/src/main.rs).
* **Dependencies**: None.
* **Tasks**:
  1. Add collapsible scroll region (`egui::ScrollArea::vertical`) inside `Panel::left` so 10+ workspaces never push `+ New Workspace` or footer buttons off-screen!
  2. Restyle workspace cards (`8 px` radius, purple border + tint on selected workspace): status dot + bold workspace name + truncated path + git branch pill + smooth glowing vector polyline sparkline (`render_sparkline_line`), keeping right-click/hover rename (`✎`) & close (`×`).
  3. Pin full-width `+ New Workspace` below list and `Settings` + `Shortcuts` side-by-side at bottom of `Panel::left` (`Shortcuts` opens `SettingsTab::Shortcuts` directly).
* **Acceptance criteria**: Sidebar spans full height cleanly, scrolls cleanly with many workspaces, footer stays pinned to bottom, and switching/renaming/closing workspaces preserves all state.
* **Rollback**: `git revert` Phase 4 commits.
* **Regression check**: Collapsed sidebar still switches workspaces via initial badges & hover tooltips; Folder Lock blocks invalid `+ New Workspace` folders cleanly.

### Phase 5: Rounded Tab Strip, Split Pane Headers & Visual Splitters
* **Goal**: Match the mockup's rounded status-dot tabs, crisp 2×2 split panes with purple active focus border, per-pane agent headers, and discoverable draggable split dividers.
* **Files touched**: [`tab_bar.rs`](../crates/agemax-app/src/tab_bar.rs), [`chrome_ui.rs`](../crates/agemax-app/src/chrome_ui.rs), [`visual_alive.rs`](../crates/agemax-app/src/visual_alive.rs), [`crates/agemax-core/src/workspace.rs`](../crates/agemax-core/src/workspace.rs), [`main.rs`](../crates/agemax-app/src/main.rs).
* **Dependencies**: None.
* **Tasks**:
  1. Upgrade [`tab_bar.rs`](../crates/agemax-app/src/tab_bar.rs) with scrollable horizontal rounded pill tabs (`>_ title`, live `AgentStatus` dot for the tab's aggregate/active pane, close `×`, `+` button).
  2. Support human-readable pane labels (`agy-1`, `agy-2`, etc. or team role label `Researcher`) cleanly in `PaneHeaderProps`, showing rounded status pills (`Running`, `Needs input`, `Done`, `Error`, `Ready`) + optional compact mascot + right-aligned vector icons (`Maximize/Restore`, `Split Right`, `Split Down`, `Close`).
  3. Paint subtle visible `4 px` gutter between split panes in `CentralPanel` and brighten divider line + hover glow when hovered/dragged; ensure focused pane shows clean purple border (`#8B5CF6`) distinct from attention states.
* **Acceptance criteria**: Splitting panes 2×2 (Right + Down) fills `CentralPanel` proportionally, dividers drag smoothly with live cursor feedback, Maximize (`Ctrl+Shift+Enter`) zooms single pane full-canvas cleanly, tabs reflect status dots.
* **Rollback**: `git revert` Phase 5 commits.
* **Regression check**: Team grid layouts, WebView2 split panes, ACP split panes, and pane exit recovery bars all behave identically.

### Phase 6: Terminal Usability, Workspace/Action Command Palette, Shortcuts & Persistence
* **Goal**: Harden terminal UX, wire workspace navigation into `CommandPalette`, align font defaults, and verify full session/config persistence across restarts.
* **Files touched**: [`command_palette.rs`](../crates/agemax-app/src/command_palette.rs), [`mouse_ux.rs`](../crates/agemax-app/src/mouse_ux.rs), [`crates/agemax-config/src/schema.rs`](../crates/agemax-config/src/schema.rs), [`main.rs`](../crates/agemax-app/src/main.rs).
* **Dependencies**: None.
* **Tasks**:
  1. Feed active `workspaces` into `CommandPalette::sync_workspaces(...)` so typing in the top Search Pill lets users jump to any workspace OR run any built-in/custom action (`"Search workspaces, run action..."`).
  2. Verify keyboard focus rules: clicking terminal gives focus immediately; opening any modal/palette/broadcast/overflow menu cleanly suppresses PTY key forwarding; plain `Ctrl+K` still forwards `\x0b` inside terminals unless user explicitly opts into `Use Ctrl+K Command Palette` in Shortcuts.
  3. Verify `Ctrl++`/`Ctrl+-`/`Ctrl+0`/`Ctrl+Wheel` font zoom updates status bar `px` pill & reflows ConPTY `cols`/`rows` seamlessly; verify copy/paste/scrollback/URL click across resized full-screen panes.
  4. Verify `session.json` + `config.toml` persist UI mode (`Minimal`/`Full`), sidebar collapse state, split ratios, active tab/pane, and allowed folder lock cleanly across restarts.
* **Acceptance criteria**: Jumping workspaces & commands via Search Pill works smoothly; font zoom & terminal selection work accurately at all split sizes; restart restores exact workspace & split tree inside allowed folders.
* **Rollback**: `git revert` Phase 6 commits.
* **Regression check**: Right-click context menu, `Ctrl+Shift+U` jump unread, `Alt+Arrows` directional focus, `session.json` restore prompt & `--conversation` resume.

### Phase 7: Optional Data-Driven Telemetry Extras (CPU/RAM Meters, Live Branch Refresh)
* **Goal**: Populate real Windows CPU % and RAM % meters in the bottom status bar and auto-refresh git branch names if `git checkout` runs inside a pane.
* **Files touched**: [`telemetry.rs`](../crates/agemax-app/src/telemetry.rs), [`status_bar.rs`](../crates/agemax-app/src/status_bar.rs), [`Cargo.toml`](../Cargo.toml) (`Win32_System_SystemInformation` feature in `windows` — zero new crates), [`main.rs`](../crates/agemax-app/src/main.rs).
* **New dependencies**: Add `"Win32_System_SystemInformation"` to existing `windows = "0.62"` workspace dependency (keeps zero third-party additions and `cargo-deny` clean).
* **Tasks**:
  1. Add throttled 2-second system sampler (`SystemStatsPoller`) calling Win32 `GlobalMemoryStatusEx` + `GetSystemTimes` delta cleanly on a background worker thread (never on UI thread).
  2. Refresh `detect_git_branch(&ws.cwd)` inside the existing 4-second `GitStatsPoller` batch so branch badges update live if the user checks out a new branch inside ConPTY.
  3. Render `CPU XX%` and `RAM XX%` mini progress pills cleanly inside `status_bar.rs`.
* **Acceptance criteria**: CPU/RAM bars update smoothly every ~2s with `< 0.2%` added idle CPU; branch switches inside panes reflect in both sidebar card & bottom bar within ~4s.
* **Rollback**: Disable/revert Phase 7 commit cleanly (bottom status bar continues showing branch/cwd/agents/encoding/font without meters).

---

## 6. Risk Register

| Risk | Likelihood | Impact | Mitigation |
| :--- | :--- | :--- | :--- |
| **1. Switching root UI to `Panel` + `CentralPanel` breaks WebView2 child `HWND` coordinates or ACP/Welcome/Grid views** | Medium | High | Migrate panels carefully in Phase 2 alone; compute `term_rect` directly in logical points multiplied by `ctx.pixels_per_point()` inside `CentralPanel`; hide WebView2 HWNDs cleanly whenever modals/Grid/Welcome/menus are open. |
| **2. Resizing ConPTY during window drag or pane animations scrambles PowerShell/CLI prompt wrapping** | High | Medium | Keep `PaneRectAnimator::allow_backend_resize` (~120 ms debounce) already built in `visual_alive.rs`; also debounce window-drag `ConPTY` resize (~60 ms) while clipping `TerminalSurface` rendering cleanly to current rect. |
| **3. Custom borderless window titlebar (`with_decorations(false)`) breaks Windows 11 Aero Snap, DWM Mica, or window shadow** | High | Medium | Keep native Win32 dark title bar (`DWMWA_USE_IMMERSIVE_DARK_MODE`) for Phases 1–6 while placing the centered Search Pill in a top header bar; evaluate borderless mode only as an explicit user choice in Open Questions. |
| **4. Font/icon fallback differences across Windows 10 (no fluent icons) vs Windows 11 & 100/125/150% DPI scaling** | Medium | High | Never depend on OS system emoji fonts (`seguiemj.ttf`); embed either an MIT icon TTF (`include_bytes!`) or crisp resolution-independent `egui::Painter` vector shapes tested at `ppp = 1.0, 1.25, 1.5`. |
| **5. Adding `Ctrl+K` search shortcut hijacks PowerShell/bash readline kill-line (`\x0b`)** | Medium | Medium | Preserve safe `Ctrl+Shift+P` default (`REQ-054`) while displaying `Use Ctrl+K Command Palette` opt-in in Settings -> Shortcuts and reflecting the active chord dynamically inside the top Search Pill placeholder! |
| **6. Multi-pane (4–9 live terminals) rendering drops below 60 fps during dense scrolling** | Medium | Medium | Preserve span batching in [`main.rs:3109-3156`](../crates/agemax-app/src/main.rs#L3109-L3156), clip invisible rows outside `term_rect`, coalesce PTY at 4 ms, and pause high-rate repaints when all panes are settled (`REQ-061`). |
| **7. Layout changes accidentally bypass `allowed_root_dir` or `enforce_agy_sandbox`** | Low | **Critical** | Path bar click still opens `SettingsTab::Safety`; keep `validate_workspace_path` unchanged across `NewWorkspaceModal`, `session.json` restore, single-instance activation, and `workspace.create` IPC. |
| **8. `cargo-deny` rejects a new icon font/crate license in CI** | Low | High | Prefer pure `egui::Painter` vector icons OR an explicitly MIT-licensed icon asset (`Lucide`) audited against `deny.toml` (`allow = ["MIT", "Apache-2.0", ...]`); use Win32 APIs via existing `windows = "0.62"` for CPU/RAM. |

---

## 7. Regression Strategy

### Automated Checks (Run After Every Phase)
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`
4. Screenshot & visual layout check launching `target/debug/agemax-app.exe` (or `release`) across three fixed window sizes:
   * `1148x745` (compact default; test overflow menu & minimum `900px` toolbar rule)
   * `1600x1000` (standard laptop/QHD scaled)
   * `1920x1080` (full HD maximized / split 2×2)
5. Automated PowerShell verification inside a pane checking dynamic ConPTY size matches pane bounds (`$Host.UI.RawUI.WindowSize` changes cleanly when resizing window from `1148x745` -> `1920x1080` and back).

### Full Manual Smoke-Test Checklist (Every Phase)
* [ ] Zero `▯` missing-glyph boxes in toolbar, sidebar, tabs, pane headers, path bar, status bar, Command Palette, Settings, and right drawer.
* [ ] Terminal fills 100% of central space cleanly at `1148x745`, `1600x1000`, and `1920x1080` with no bottom empty void; OS resize updates PTY rows/cols cleanly.
* [ ] Single-row toolbar stays on one line down to `900 px` width (overflow menu catches extra items cleanly).
* [ ] **Workspaces**: Create Local/WSL/SSH workspace, switch workspaces, rename (`✎`), close (`×`), collapse/expand sidebar.
* [ ] **Safety / Folder Lock**: Lock folder via Path bar -> `Settings -> Safety & Folders`; verify outside directory is rejected in New Workspace & IPC; verify `agy --sandbox` flag toggle works.
* [ ] **Tabs & Splits**: New Tab (`Ctrl+Shift+T`), rename/close tab, Split Right (`Ctrl+Shift+D`), Split Down (`Ctrl+Shift+R`), drag divider bars, Maximize/Zoom (`Ctrl+Shift+Enter`), directional focus (`Alt+Arrows`), exited pane recovery banner.
* [ ] **Agent & Mission Features**: `▶ Start agy` launcher (`New Session`, `Plan Mode`, `Continue Last`), `🤖 ACP` card panel, `👥 Team` grid launch, `⊞ Grid` overview, `📣 Broadcast` confirmed multi-pane send, `🏠 Home` welcome view, live `Working`/`Thinking`/`RunningTool`/`Waiting`/`Done`/`Failed` borders & mascots.
* [ ] **Alerts & Drawer**: `🔔 Alerts` / `📡 Feed` right drawer opens cleanly without overlapping terminal; WinRT Toast, Taskbar Flash, Chime, `Ctrl+Shift+U` jump unread work.
* [ ] **Browser Preview**: `🌐 Preview` opens WebView2 cleanly, aligns to pane rect, hides cleanly when modal/Grid/Welcome opens, and switches back to terminal.
* [ ] **Terminal Input & Persistence**: Typing, `Ctrl+C`, `Ctrl+V`/`Ctrl+Shift+C`/`Ctrl+Shift+V`, wheel scrollback, click-drag selection, `Ctrl+Click` link, `Ctrl+`/`Ctrl-`/`Ctrl+0` font zoom, `View: Minimal | Full` toggle, config hot reload, restart restores `session.json` + `config.toml`.

---

## 8. Branching & Commit Strategy

1. Create **one long-lived feature branch** from `develop` for the entire UI upgrade:
   * `git checkout develop && git pull origin develop`
   * `git checkout -b feat/ui-upgrade`
   * `git tag baseline-pre-ui-upgrade`
2. Tag before each phase (`pre-phase-1`, `pre-phase-2`, ..., `pre-phase-7`) so bisect/rollback is instantaneous.
3. Make **one atomic, compilable commit per task** using Conventional Commits (e.g., `feat(ui): add vector UiIcon set and replace missing Unicode glyphs`, `fix(layout): migrate AgeMaxApp::ui to egui Panel and CentralPanel for full-height ConPTY resize`).
4. Never commit directly to `main`; once all phases pass verification + smoke tests, open a PR targeting `develop` and preserve the feature branch on GitHub.

---

## 9. Open Questions (Need Your Input Before Implementation)

1. **Title Bar Style**: Do you want a **borderless custom title bar** (`with_decorations(false)` matching the mockup's purple logo + search pill + `— □ ×`, requiring custom Win32 drag/resize handling), OR should we **keep the native Windows dark title bar** and put the centered Search Pill in the top header strip underneath it? *(Recommended: Keep native dark title bar first so Aero Snap, Mica, and Windows 10/11 window shadows never glitch, then optionally add borderless later.)*
2. **Icon Implementation**: Would you like **embedded MIT Lucide monochrome icon font** (exact crisp Feather/Lucide icons matching the mockup) or **pure Rust `egui::Painter` geometric icons** (zero font binary needed)? *(Note: both are MIT-compatible and have zero runtime file dependencies.)*
3. **CPU/RAM Meters**: In Phase 7, do you prefer native Win32 API calls (`GlobalMemoryStatusEx` + `GetSystemTimes` via the existing `windows` crate -> 0 new dependencies) or the `sysinfo` crate (cross-platform ready)?
4. **Search Pill Shortcut Label**: The mockup displays `Ctrl+K` in the top Search Pill, whereas Age Max currently reserves plain `Ctrl+K` inside focused terminals (`\x0b` kill-line, `REQ-054`) and defaults Command Palette to `Ctrl+Shift+P` (with `Use Ctrl+K` opt-in in Settings). Should the search pill dynamically display whichever shortcut is configured (default `Ctrl+Shift+P`), or switch the default to `Ctrl+K`?
5. **Minimum Supported Window Size**: Right now `main.rs:104` sets `with_min_inner_size([450.0, 320.0])`. Can we raise the recommended minimum size slightly (e.g. `720x480` or `900x540` with responsive `···` toolbar overflow down to `900px`)?
6. **Agent Mascots vs Minimal Headers**: In the mockup, pane headers show `>_ agy-1 [● Running]` without the vector robot mascot inside the pane header. Should `show_agent_mascots` default to `false` on pane headers (available in Settings / Grid View) or stay enabled alongside the new pill badge?

---

## 10. Definition of Done

The UI upgrade is complete only when every item below holds true across `1148x745`, `1600x1000`, and `1920x1080` on Windows at 100%, 125%, and 150% DPI:
* [ ] **Zero missing-glyph boxes (`▯`)** anywhere in chrome, modals, sidebar, tabs, or status badges.
* [ ] **Full-viewport terminal sizing**: Terminal canvas claims 100% of available center height and width (no ~70% empty void); ConPTY `cols`/`rows` updates accurately on window resize and pane splits.
* [ ] **Single-row responsive toolbar**: Toolbar stays on one row cleanly down to `900 px` width using grouped icons, primary `Start agy` CTA, and an overflow menu below tight widths.
* [ ] **All existing features preserved & passing**: ConPTY/WSL/SSH shells, `Start agy`, Split Right/Down, Drag Splitters, Zoom, Grid View, ACP Card Panel, Team Grid, Safe Broadcast, Welcome View, WebView2 Preview, Alerts & Feed Drawer, Folder Lock (`allowed_root_dir`), `--sandbox`, Toast/Taskbar/Chime alerts, CLI IPC, and 6 themes.
* [ ] **Persistence across restarts**: UI chrome mode (`Minimal`/`Full`), sidebar collapse, theme, shortcuts, split tree, workspaces, and folder lock survive restart cleanly via `config.toml` and `session.json`.
* [ ] **Clean CI gates**: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cargo-deny check licenses`, and `cargo build --release --workspace` all exit 0.
