#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_icon;
mod backdrop_win;
mod chrome_ui;
mod command_palette;
mod folder_picker;
mod icons;
mod ipc_bridge;
mod mission_control;
mod mouse_ux;
mod notification_win;
mod onboarding_settings;
mod sidebar;
mod status_bar;
mod stretch_panels;
mod tab_bar;
mod telemetry;
mod ui_tokens;
mod visual_alive;
mod webview_pane;

use agemax_agy::acp::{AcpEvent, LiveAcpSession};
use agemax_agy::adapter::{AgentAdapter, AgentLaunchPreset, AntigravityAdapter};
use agemax_agy::osc::{AgentOscEvent, OscParser};
use agemax_agy::profile::{ProfileConfig, ProfileManager};
use agemax_config::hot_reload::ConfigWatcher;
use agemax_config::keybindings::{KeyChord, KeyCode};
use agemax_config::safety::{
    display_clean_path, resolve_startup_directory, validate_workspace_path,
};
use agemax_config::schema::{AgeMaxConfig, UiMode};
use agemax_core::layout::{Direction, LayoutTree, PaneId, SplitDirection, WorkspaceId};
use agemax_core::model::{Point, Rect as CoreRect, TeamRoleMeta, TerminalSize};
use agemax_core::notification::{ActivityFeed, NotificationStore};
use agemax_core::session::{
    LayoutSnapshot, PaneSnapshot, SessionSnapshot, TabSnapshot, WorkspaceSnapshot,
};
use agemax_core::status::AgentStatus;
use agemax_core::workspace::{Tab, Workspace, WorkspaceBackend};
use agemax_ipc::activation::{try_activate_existing_instance, LaunchArgs};
use agemax_ipc::security::get_default_pipe_name;
use agemax_term::backend_from_workspace;
use agemax_term::metadata::detect_git_branch;
use agemax_term::surface::TerminalSurface;
use agemax_term::vt::GridPoint;
use agemax_term::PtySystemBackend;
use backdrop_win::BackdropController;
use chrome_ui::{
    render_header_strip, render_pane_header, render_top_toolbar, render_workspace_folder_bar,
    NewWorkspaceModal, NewWorkspaceRequest, PaneHeaderAction, PaneHeaderProps, RenameModal,
    RenameTarget, ToolbarAction,
};
use command_palette::{CommandPalette, PaletteAction};
use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Stroke, StrokeKind, Vec2};
use ipc_bridge::{start_ipc_server, IpcAppCommand};
use mission_control::{
    render_agents_grid, render_right_drawer, render_welcome_view, BroadcastModal,
    BroadcastSubmitRequest, BroadcastTarget, GridPaneCard, GridViewAction, RightDrawerAction,
    RightDrawerTab, WelcomeAction,
};
use mouse_ux::{
    color32_from_rgb, render_context_menu, render_pane_exit_banner, render_session_restore_banner,
    viewport_cell_at_pos, ActiveSplitDrag, ContextMenuAction, ContextMenuState, ExitBannerAction,
    SessionRestoreAction, TextSelection,
};
use notification_win::{NotificationService, PlatformNotificationService};
use onboarding_settings::{
    render_onboarding_banner, AgentLaunchRequest, AgentLauncherModal, OnboardingAction,
    PreflightState, SettingsModal, SettingsTab,
};
use sidebar::{render_sidebar, SidebarAction, SidebarProps, SidebarWorkspaceMeta};
use status_bar::{render_status_bar, StatusBarAction, StatusBarProps};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use stretch_panels::{
    render_acp_pane, team_badge_color, AcpPaneAction, AcpPaneState, TeamLaunchRequest,
    TeamLauncherModal,
};
use tab_bar::{render_tab_bar, TabBarAction};
use telemetry::{infer_pty_tool_hint, GitStatsPoller, PaneTelemetry};
use uuid::Uuid;
use visual_alive::{PaneRectAnimator, PaneVisualAnimator};
use webview_pane::{render_browser_pane, BrowserPaneAction, WebViewSurface};

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let launch_args = LaunchArgs::parse_from(std::env::args_os());
    let pipe_name = launch_args
        .pipe
        .clone()
        .unwrap_or_else(get_default_pipe_name);

    if try_activate_existing_instance(&pipe_name, launch_args.cwd.as_deref()) {
        tracing::info!("Forwarded activation to running Age Max instance on {pipe_name}");
        return Ok(());
    }

    let initial_cfg = AgeMaxConfig::load_or_create_default();
    let initial_size = parse_capture_inner_size().unwrap_or([1150.0, 720.0]);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Age Max - Native Agent Multiplexer")
            .with_icon(app_icon::window_icon())
            .with_transparent(initial_cfg.general.enable_mica_backdrop)
            .with_inner_size(initial_size)
            .with_min_inner_size([900.0, 540.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AgeMax",
        native_options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(AgeMaxApp::new_with_launch(
                Some(cc),
                pipe_name,
                launch_args,
            )))
        }),
    )
}

fn parse_capture_inner_size() -> Option<[f32; 2]> {
    let raw = std::env::var("AGEMAX_CAPTURE_SIZE").ok()?;
    let (w_str, h_str) = raw.split_once('x')?;
    let w: f32 = w_str.trim().parse().ok()?;
    let h: f32 = h_str.trim().parse().ok()?;
    Some([w.max(320.0), h.max(240.0)])
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Register embedded Lucide icon font in its own dedicated font family
    icons::register_lucide_font(&mut fonts);

    // On Windows, prefer Segoe UI for proportional UI text if available
    if let Ok(ui_bytes) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert(
            "Segoe UI".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(ui_bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Segoe UI".to_owned());
    }

    // On Windows, attempt to load Cascadia Code or Consolas if present
    for path in &[
        "C:\\Windows\\Fonts\\CascadiaCode.ttf",
        "C:\\Windows\\Fonts\\cascadia.ttf",
        "C:\\Windows\\Fonts\\consola.ttf",
    ] {
        if let Ok(font_bytes) = std::fs::read(path) {
            let font_name = if path.contains("consol") {
                "Consolas"
            } else {
                "Cascadia Code"
            };
            fonts.font_data.insert(
                font_name.to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(font_bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, font_name.to_owned());
            break;
        }
    }

    ctx.set_fonts(fonts);
}

pub struct AgeMaxApp {
    workspaces: Vec<Workspace>,
    active_workspace_index: usize,
    surfaces: HashMap<PaneId, TerminalSurface>,
    webview_panes: HashMap<PaneId, WebViewSurface>,
    acp_panes: HashMap<PaneId, AcpPaneState>,
    pane_team_roles: HashMap<PaneId, TeamRoleMeta>,
    osc_parsers: HashMap<PaneId, OscParser>,
    pane_status: HashMap<PaneId, AgentStatus>,
    agent_conversation_ids: HashMap<PaneId, String>,
    pane_telemetry: HashMap<PaneId, PaneTelemetry>,
    git_stats_poller: GitStatsPoller,
    notification_store: NotificationStore,
    activity_feed: ActivityFeed,
    notification_service: Box<dyn NotificationService>,
    ipc_rx: crossbeam_channel::Receiver<IpcAppCommand>,
    config: AgeMaxConfig,
    config_watcher: Option<ConfigWatcher>,
    command_palette: CommandPalette,
    new_workspace_modal: NewWorkspaceModal,
    rename_modal: RenameModal,
    settings_modal: SettingsModal,
    agent_launcher_modal: AgentLauncherModal,
    team_launcher_modal: TeamLauncherModal,
    broadcast_modal: BroadcastModal,
    preflight: PreflightState,
    pending_restore: Option<SessionSnapshot>,
    text_selection: Option<TextSelection>,
    active_split_drag: Option<ActiveSplitDrag>,
    context_menu: ContextMenuState,
    last_theme: Option<agemax_config::theme::ResolvedTheme>,
    visual_animator: PaneVisualAnimator,
    rect_animator: PaneRectAnimator,
    backdrop_ctrl: BackdropController,
    any_pane_animating: bool,
    pending_window_activate: bool,
    host_hwnd: Option<isize>,
    show_notifications_panel: bool,
    right_drawer_tab: RightDrawerTab,
    show_grid_view: bool,
    show_welcome_view: bool,
    is_sidebar_collapsed: bool,
    font_size: f32,
    char_width: f32,
    char_height: f32,
    start_time: Instant,
    first_frame_rendered: bool,
    capture_frames_left: Option<u8>,
    capture_out_path: Option<PathBuf>,
    capture_done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Default for AgeMaxApp {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AgeMaxApp {
    pub fn new(cc: Option<&eframe::CreationContext>) -> Self {
        Self::new_with_launch(cc, get_default_pipe_name(), LaunchArgs::default())
    }

    pub fn new_with_launch(
        cc: Option<&eframe::CreationContext>,
        pipe_name: String,
        launch_args: LaunchArgs,
    ) -> Self {
        let start_time = Instant::now();
        let initial_pane = PaneId::new();
        let explicit_dir = launch_args.cwd.filter(|p| p.is_dir());

        let mut config = AgeMaxConfig::load_or_create_default();
        let blocked_startup_dir = explicit_dir.as_ref().and_then(|d| {
            validate_workspace_path(d, config.general.allowed_root_dir.as_deref()).err()
        });
        let cwd = resolve_startup_directory(explicit_dir.as_deref(), &config.general);
        config.merge_workspace_config(&cwd);

        let ws_name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "Main Workspace".to_string());
        let git_branch = detect_git_branch(&cwd);

        let mut initial_workspace = Workspace::new(ws_name, cwd.clone(), initial_pane);
        initial_workspace.git_branch = git_branch;

        let initial_surface = spawn_terminal_surface(
            TerminalSize::new(80, 24),
            &cwd,
            config.general.default_shell.as_deref(),
            HashMap::new(),
        )
        .expect("Failed to initialize initial ConPTY surface");

        let mut surfaces = HashMap::new();
        surfaces.insert(initial_pane, initial_surface);

        let mut osc_parsers = HashMap::new();
        osc_parsers.insert(initial_pane, OscParser::new());

        let mut pane_status = HashMap::new();
        pane_status.insert(initial_pane, AgentStatus::Idle);

        let (ipc_tx, ipc_rx) = crossbeam_channel::unbounded();
        let mut preflight = PreflightState::new();
        if let Some(ctx_container) = cc {
            start_ipc_server(pipe_name, ipc_tx, ctx_container.egui_ctx.clone());
            preflight.trigger_check(
                config.general.default_shell.clone(),
                Some(ctx_container.egui_ctx.clone()),
            );
        } else {
            preflight.trigger_check(config.general.default_shell.clone(), None);
        }

        let config_watcher = ConfigWatcher::watch(AgeMaxConfig::default_path()).ok();
        let mut command_palette = CommandPalette::new();
        command_palette.sync_builtins(&config.keybindings);
        command_palette.set_custom_commands(&config.commands);

        let initial_font_size = config.general.font_size.clamp(9.0, 32.0);
        let pending_restore = SessionSnapshot::load_from_file(&SessionSnapshot::default_path())
            .ok()
            .filter(|snap| !snap.workspaces.is_empty());

        let mut notification_store = NotificationStore::new();
        let mut show_notifications_panel = false;
        if let Some(err) = blocked_startup_dir {
            notification_store.notify(
                initial_workspace.id,
                initial_pane,
                AgentStatus::Failed,
                format!("Startup folder blocked ({err}) — opened safe folder instead."),
            );
            show_notifications_panel = true;
        }

        Self {
            workspaces: vec![initial_workspace],
            active_workspace_index: 0,
            surfaces,
            webview_panes: HashMap::new(),
            acp_panes: HashMap::new(),
            pane_team_roles: HashMap::new(),
            osc_parsers,
            pane_status,
            agent_conversation_ids: HashMap::new(),
            pane_telemetry: HashMap::new(),
            git_stats_poller: GitStatsPoller::spawn(cc.map(|c| c.egui_ctx.clone())),
            notification_store,
            activity_feed: ActivityFeed::new(),
            notification_service: Box::new(PlatformNotificationService::new()),
            ipc_rx,
            config,
            config_watcher,
            command_palette,
            new_workspace_modal: NewWorkspaceModal::new(),
            rename_modal: RenameModal::default(),
            settings_modal: SettingsModal::default(),
            agent_launcher_modal: AgentLauncherModal::new(),
            team_launcher_modal: TeamLauncherModal::new(),
            broadcast_modal: BroadcastModal::default(),
            preflight,
            pending_restore,
            text_selection: None,
            active_split_drag: None,
            context_menu: ContextMenuState::default(),
            last_theme: None,
            visual_animator: PaneVisualAnimator::new(),
            rect_animator: PaneRectAnimator::new(),
            backdrop_ctrl: BackdropController::default(),
            any_pane_animating: false,
            pending_window_activate: false,
            host_hwnd: None,
            show_notifications_panel,
            right_drawer_tab: RightDrawerTab::default(),
            show_grid_view: false,
            show_welcome_view: false,
            is_sidebar_collapsed: false,
            font_size: initial_font_size,
            char_width: 8.4,
            char_height: 18.0,
            start_time,
            first_frame_rendered: false,
            capture_out_path: std::env::var_os("AGEMAX_CAPTURE_SCREENSHOT")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            capture_frames_left: std::env::var_os("AGEMAX_CAPTURE_SCREENSHOT")
                .filter(|v| !v.is_empty())
                .map(|_| 24),
            capture_done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn is_modal_open(&self) -> bool {
        self.command_palette.is_open
            || self.new_workspace_modal.is_open
            || self.rename_modal.is_open
            || self.settings_modal.is_open
            || self.agent_launcher_modal.is_open
            || self.team_launcher_modal.is_open
            || self.broadcast_modal.is_open
    }

    fn remove_pane_resources(&mut self, pane_id: PaneId) {
        let resolved_theme = self.config.theme.resolved_rgb();
        self.rect_animator.start_close(
            pane_id,
            color32_from_rgb(resolved_theme.background),
            color32_from_rgb(resolved_theme.active_border),
            Instant::now(),
        );
        self.surfaces.remove(&pane_id);
        self.webview_panes.remove(&pane_id);
        self.acp_panes.remove(&pane_id);
        self.pane_team_roles.remove(&pane_id);
        self.osc_parsers.remove(&pane_id);
        self.pane_status.remove(&pane_id);
        self.agent_conversation_ids.remove(&pane_id);
        self.pane_telemetry.remove(&pane_id);
        self.visual_animator.remove_pane(pane_id);
    }

    fn record_feed_for_pane(
        &mut self,
        pane_id: PaneId,
        category: &str,
        summary: impl Into<String>,
    ) {
        for ws in &self.workspaces {
            if ws.tabs.iter().any(|t| t.layout.panes().contains(&pane_id)) {
                self.activity_feed
                    .push(ws.id, ws.name.clone(), pane_id, category, summary);
                return;
            }
        }
    }

    fn gather_broadcast_targets(&self) -> Vec<BroadcastTarget> {
        let mut out = Vec::new();
        for ws in &self.workspaces {
            let (backend_label, folder_display) = match &ws.backend {
                WorkspaceBackend::Local => ("Local".to_string(), display_clean_path(&ws.cwd)),
                WorkspaceBackend::Wsl { distro, linux_cwd } => (
                    format!("WSL:{}", distro.as_deref().unwrap_or("default")),
                    linux_cwd.as_deref().unwrap_or("~").to_string(),
                ),
                WorkspaceBackend::Ssh {
                    target, remote_dir, ..
                } => (
                    format!("SSH:{target}"),
                    remote_dir.as_deref().unwrap_or("~").to_string(),
                ),
            };
            for tab in &ws.tabs {
                for pane_id in tab.layout.panes() {
                    let is_webview = self.webview_panes.contains_key(&pane_id);
                    let is_ready = if is_webview {
                        false
                    } else if let Some(AcpPaneState::Running(sess)) = self.acp_panes.get(&pane_id) {
                        sess.state.session_id.is_some()
                    } else if let Some(surf) = self.surfaces.get(&pane_id) {
                        !surf.has_exited()
                    } else {
                        false
                    };
                    let status = self
                        .pane_status
                        .get(&pane_id)
                        .copied()
                        .unwrap_or(AgentStatus::Idle);
                    out.push(BroadcastTarget {
                        pane_id,
                        workspace_name: ws.name.clone(),
                        tab_title: tab.title.clone(),
                        folder_display: folder_display.clone(),
                        backend_label: backend_label.clone(),
                        is_ready,
                        status,
                    });
                }
            }
        }
        out
    }

    fn execute_broadcast(&mut self, req: BroadcastSubmitRequest) {
        let trimmed = req.prompt.trim();
        if trimmed.is_empty() {
            return;
        }
        let mut delivered = 0usize;
        for pid in req.target_panes {
            if let Some(AcpPaneState::Running(sess)) = self.acp_panes.get_mut(&pid) {
                if sess.send_prompt(trimmed).is_ok() {
                    delivered += 1;
                    self.pane_status.insert(pid, AgentStatus::Thinking);
                }
            } else if let Some(surface) = self.surfaces.get_mut(&pid) {
                if !surface.has_exited() {
                    let payload = if req.append_enter {
                        format!("{trimmed}\r")
                    } else {
                        trimmed.to_string()
                    };
                    if surface.send_input(payload.as_bytes()).is_ok() {
                        delivered += 1;
                    }
                }
            }
        }
        if let Some(ws) = self.active_workspace() {
            if let Some(pid) = ws.active_pane_id() {
                self.activity_feed.push(
                    ws.id,
                    ws.name.clone(),
                    pid,
                    "broadcast",
                    format!("Sent prompt to {delivered} session(s)"),
                );
            }
        }
    }

    fn launch_quick_preset_in_active_pane(&mut self, preset: AgentLaunchPreset) {
        if let Some(pane_id) = self.active_pane_id() {
            self.launch_agent_request(AgentLaunchRequest {
                target_pane: pane_id,
                preset,
                profile: ProfileConfig::default_mode_a(),
                sandbox: self.config.general.enforce_agy_sandbox,
            });
            self.show_welcome_view = false;
        }
    }

    fn spawn_surface_for_backend(
        &self,
        backend: &WorkspaceBackend,
        size: TerminalSize,
        cwd: &Path,
        extra_env: HashMap<String, String>,
    ) -> Result<TerminalSurface, agemax_term::PtyError> {
        let pty_backend = backend_from_workspace(backend);
        let pty_cfg = pty_backend.build_pty_config(
            cwd,
            self.config.general.default_shell.as_deref(),
            size,
            extra_env,
        );
        TerminalSurface::new(size, pty_cfg)
    }

    fn workspace_backend_for_pane(&self, pane_id: PaneId) -> WorkspaceBackend {
        self.find_workspace_for_pane(pane_id)
            .and_then(|ws_id| self.workspaces.iter().find(|w| w.id == ws_id))
            .map(|w| w.backend.clone())
            .unwrap_or_default()
    }

    fn dispatch_attention_alerts(&self, ws_id: WorkspaceId) {
        if self.config.general.enable_toast_notifications {
            if let Some(item) = self.notification_store.latest_unread_for_workspace(ws_id) {
                self.notification_service.show_notification(item);
            }
        }
        if self.config.general.enable_taskbar_flash {
            self.notification_service.flash_window();
        }
        if self.config.general.enable_sound_notifications {
            self.notification_service.play_chime();
        }
    }

    fn spawn_default_surface(
        &self,
        size: TerminalSize,
        cwd: &Path,
        extra_env: HashMap<String, String>,
    ) -> Result<TerminalSurface, agemax_term::PtyError> {
        self.spawn_surface_for_backend(&WorkspaceBackend::Local, size, cwd, extra_env)
    }

    fn copy_selection_to_clipboard(
        &self,
        ctx: &egui::Context,
        target_pane: Option<PaneId>,
    ) -> bool {
        let Some(sel) = self.text_selection else {
            return false;
        };
        if !sel.is_non_empty() {
            return false;
        }
        if let Some(pid) = target_pane {
            if sel.pane_id != pid {
                return false;
            }
        }
        let Some(surface) = self.surfaces.get(&sel.pane_id) else {
            return false;
        };
        let text = surface.extract_selection_text(sel.anchor, sel.end);
        if text.is_empty() {
            return false;
        }
        ctx.copy_text(text);
        true
    }

    fn adjust_font_zoom(&mut self, delta: f32) {
        let next = (self.font_size + delta).clamp(9.0, 32.0);
        if (next - self.font_size).abs() > 0.1 {
            self.font_size = next;
            self.config.general.font_size = next;
            let _ = self.config.save_default();
        }
    }

    fn restart_pane_shell(&mut self, pane_id: PaneId) -> bool {
        let cwd = self
            .find_workspace_for_pane(pane_id)
            .and_then(|ws_id| self.workspaces.iter().find(|w| w.id == ws_id))
            .map(|w| w.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let backend = self.workspace_backend_for_pane(pane_id);
        let size = self
            .surfaces
            .get(&pane_id)
            .map(|s| TerminalSize::new(s.cols() as u16, s.rows() as u16))
            .unwrap_or_else(|| TerminalSize::new(80, 24));

        if let Ok(fresh) = self.spawn_surface_for_backend(&backend, size, &cwd, HashMap::new()) {
            self.webview_panes.remove(&pane_id);
            self.acp_panes.remove(&pane_id);
            self.surfaces.insert(pane_id, fresh);
            self.osc_parsers.insert(pane_id, OscParser::new());
            self.pane_status.insert(pane_id, AgentStatus::Idle);
            true
        } else {
            false
        }
    }

    fn launch_agent_request(&mut self, req: AgentLaunchRequest) {
        let pane_id = req.target_pane;
        self.focus_pane_in_active_tab(pane_id);

        let cwd = self
            .find_workspace_for_pane(pane_id)
            .and_then(|ws_id| self.workspaces.iter().find(|w| w.id == ws_id))
            .map(|w| w.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
            if let Some(ws) = self.workspaces.iter_mut().find(|w| w.id == ws_id) {
                ws.profile_label = if req.profile.isolate_storage {
                    Some(req.profile.name.clone())
                } else {
                    None
                };
            }
        }

        let adapter = AntigravityAdapter::new();
        if adapter.detect_binary().is_none() {
            if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
                self.notification_store.notify(
                    ws_id,
                    pane_id,
                    AgentStatus::Failed,
                    "agy.exe not found. Click 'Copy Install Command' in the top guide.",
                );
                self.show_notifications_panel = true;
            }
            return;
        }

        let is_exited = self
            .surfaces
            .get(&pane_id)
            .map(|s| s.has_exited())
            .unwrap_or(true);
        if req.profile.isolate_storage || is_exited {
            let env = adapter
                .build_preset_command_with_options(
                    req.preset.clone(),
                    &req.profile,
                    &cwd,
                    req.sandbox,
                )
                .map(|c| c.env)
                .unwrap_or_default();
            let size = self
                .surfaces
                .get(&pane_id)
                .map(|s| TerminalSize::new(s.cols() as u16, s.rows() as u16))
                .unwrap_or_else(|| TerminalSize::new(80, 24));
            if let Ok(fresh) = self.spawn_default_surface(size, &cwd, env) {
                self.surfaces.insert(pane_id, fresh);
                self.osc_parsers.insert(pane_id, OscParser::new());
                self.pane_status.insert(pane_id, AgentStatus::Idle);
            }
        }

        let line = AntigravityAdapter::shell_line_for_preset_with_options(&req.preset, req.sandbox);
        if let Some(surface) = self.surfaces.get_mut(&pane_id) {
            let cmd = format!("{}\r", line);
            let _ = surface.send_input(cmd.as_bytes());
        }
        self.save_session();
    }

    fn resume_agent_in_pane(&mut self, pane_id: PaneId) {
        if self
            .surfaces
            .get(&pane_id)
            .map(|s| s.has_exited())
            .unwrap_or(false)
        {
            self.restart_pane_shell(pane_id);
        }
        let Some(conv_id) = self.agent_conversation_ids.get(&pane_id).cloned() else {
            self.launch_agy_in_pane(pane_id);
            return;
        };
        if !is_safe_conversation_id(&conv_id) {
            return;
        }
        if let Some(surface) = self.surfaces.get_mut(&pane_id) {
            let cmd = format!("agy --conversation {}\r", conv_id);
            let _ = surface.send_input(cmd.as_bytes());
        }
    }

    fn restore_session_snapshot(&mut self, snapshot: SessionSnapshot, auto_resume_agents: bool) {
        let fallback_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut restored_workspaces = Vec::new();
        let mut new_surfaces = HashMap::new();
        let mut new_parsers = HashMap::new();
        let mut new_statuses = HashMap::new();
        let mut new_conv_ids = HashMap::new();
        let mut resume_targets = Vec::new();

        for ws_snap in snapshot.workspaces {
            let ws_dir = if ws_snap.cwd.exists() {
                ws_snap.cwd.clone()
            } else {
                fallback_dir.clone()
            };
            if matches!(ws_snap.backend, WorkspaceBackend::Local)
                && validate_workspace_path(&ws_dir, self.config.general.allowed_root_dir.as_deref())
                    .is_err()
            {
                continue;
            }
            let mut tabs = Vec::new();
            for tab_snap in ws_snap.tabs {
                let mut layout = LayoutTree::from(&tab_snap.layout);
                let mut active_pane = PaneId(tab_snap.active_pane_id);
                let pane_ids = layout.panes();

                if pane_ids.is_empty() {
                    let pid = PaneId::new();
                    layout = LayoutTree::new_leaf(pid);
                    active_pane = pid;
                    if let Ok(surf) = self.spawn_surface_for_backend(
                        &ws_snap.backend,
                        TerminalSize::new(80, 24),
                        &ws_dir,
                        HashMap::new(),
                    ) {
                        new_surfaces.insert(pid, surf);
                        new_parsers.insert(pid, OscParser::new());
                        new_statuses.insert(pid, AgentStatus::Idle);
                    }
                } else {
                    for pid in &pane_ids {
                        let pane_snap = tab_snap.panes.iter().find(|p| p.id == pid.0);
                        let p_dir = pane_snap
                            .map(|p| p.cwd.clone())
                            .filter(|d| d.exists())
                            .unwrap_or_else(|| ws_dir.clone());
                        if let Ok(surf) = self.spawn_surface_for_backend(
                            &ws_snap.backend,
                            TerminalSize::new(80, 24),
                            &p_dir,
                            HashMap::new(),
                        ) {
                            new_surfaces.insert(*pid, surf);
                            new_parsers.insert(*pid, OscParser::new());
                            new_statuses.insert(*pid, AgentStatus::Idle);
                            if let Some(cid) =
                                pane_snap.and_then(|p| p.agent_conversation_id.clone())
                            {
                                new_conv_ids.insert(*pid, cid);
                                if auto_resume_agents {
                                    resume_targets.push(*pid);
                                }
                            }
                        }
                    }
                    if !pane_ids.contains(&active_pane) {
                        active_pane = pane_ids[0];
                    }
                }

                tabs.push(Tab {
                    id: agemax_core::layout::TabId::new(),
                    title: tab_snap.name,
                    layout,
                    active_pane,
                    zoomed_pane: None,
                });
            }

            if tabs.is_empty() {
                let pid = PaneId::new();
                tabs.push(Tab::new("Terminal 1", pid));
                if let Ok(surf) = self.spawn_surface_for_backend(
                    &ws_snap.backend,
                    TerminalSize::new(80, 24),
                    &ws_dir,
                    HashMap::new(),
                ) {
                    new_surfaces.insert(pid, surf);
                    new_parsers.insert(pid, OscParser::new());
                    new_statuses.insert(pid, AgentStatus::Idle);
                }
            }

            let active_tab_index = ws_snap.active_tab_index.min(tabs.len().saturating_sub(1));
            restored_workspaces.push(Workspace {
                id: WorkspaceId(ws_snap.id),
                name: ws_snap.name,
                cwd: ws_dir.clone(),
                git_branch: if matches!(ws_snap.backend, WorkspaceBackend::Local) {
                    detect_git_branch(&ws_dir)
                } else {
                    None
                },
                listening_ports: Vec::new(),
                profile_label: ws_snap.profile,
                backend: ws_snap.backend,
                tabs,
                active_tab_index,
            });
        }

        if restored_workspaces.is_empty() {
            return;
        }

        self.workspaces = restored_workspaces;
        self.active_workspace_index = snapshot
            .active_workspace_index
            .min(self.workspaces.len().saturating_sub(1));
        self.surfaces = new_surfaces;
        self.osc_parsers = new_parsers;
        self.pane_status = new_statuses;
        self.agent_conversation_ids = new_conv_ids;
        self.pending_restore = None;
        self.text_selection = None;

        for pid in resume_targets {
            self.resume_agent_in_pane(pid);
        }
        self.save_session();
    }

    fn open_new_workspace_modal(&mut self) {
        let default_dir = self
            .active_workspace()
            .map(|w| w.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let next_idx = self.workspaces.len() + 1;
        self.new_workspace_modal.open(
            &default_dir,
            next_idx,
            self.config.general.allowed_root_dir.as_deref(),
        );
    }

    fn create_workspace_in_dir(
        &mut self,
        name: String,
        cwd: PathBuf,
        launch_agy: bool,
    ) -> Option<WorkspaceId> {
        self.create_workspace_with_backend(NewWorkspaceRequest {
            name,
            cwd,
            launch_agy,
            backend: WorkspaceBackend::Local,
        })
    }

    fn create_workspace_with_backend(
        &mut self,
        mut req: NewWorkspaceRequest,
    ) -> Option<WorkspaceId> {
        if matches!(req.backend, WorkspaceBackend::Local) {
            match validate_workspace_path(&req.cwd, self.config.general.allowed_root_dir.as_deref())
            {
                Ok(safe_dir) => {
                    req.cwd = safe_dir;
                }
                Err(err) => {
                    if let (Some(ws), Some(pane)) = (self.active_workspace(), self.active_pane_id())
                    {
                        self.notification_store.notify(
                            ws.id,
                            pane,
                            AgentStatus::Failed,
                            err.to_string(),
                        );
                        self.show_notifications_panel = true;
                    }
                    return None;
                }
            }
        }

        let pane_id = PaneId::new();
        let mut ws =
            Workspace::with_backend(req.name, req.cwd.clone(), pane_id, req.backend.clone());
        if matches!(req.backend, WorkspaceBackend::Local) {
            ws.git_branch = detect_git_branch(&req.cwd);
        }

        let surface = self
            .spawn_surface_for_backend(
                &req.backend,
                TerminalSize::new(80, 24),
                &req.cwd,
                HashMap::new(),
            )
            .ok()?;
        let ws_id = ws.id;

        self.surfaces.insert(pane_id, surface);
        self.osc_parsers.insert(pane_id, OscParser::new());
        self.pane_status.insert(pane_id, AgentStatus::Idle);
        self.workspaces.push(ws);
        self.active_workspace_index = self.workspaces.len() - 1;
        if matches!(req.backend, WorkspaceBackend::Local) {
            self.config.merge_workspace_config(&req.cwd);
            self.command_palette
                .set_custom_commands(&self.config.commands);
        }
        self.save_session();

        if req.launch_agy {
            self.launch_agy_in_pane(pane_id);
        }
        Some(ws_id)
    }

    fn open_browser_in_pane(&mut self, pane_id: Option<PaneId>, url: &str) -> Option<PaneId> {
        let target = if let Some(pid) = pane_id {
            pid
        } else {
            self.split_active_pane_returning(SplitDirection::Horizontal)
                .or_else(|| self.active_pane_id())?
        };
        self.acp_panes.remove(&target);
        if let Some(existing) = self.webview_panes.get_mut(&target) {
            existing.navigate(url);
        } else {
            self.webview_panes.insert(target, WebViewSurface::new(url));
        }
        self.focus_pane_in_active_tab(target);
        Some(target)
    }

    fn open_acp_in_pane(&mut self, pane_id: PaneId) {
        let cwd = self
            .find_workspace_for_pane(pane_id)
            .and_then(|ws_id| self.workspaces.iter().find(|w| w.id == ws_id))
            .map(|w| w.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        self.webview_panes.remove(&pane_id);
        let state = match LiveAcpSession::spawn(&ProfileConfig::default_mode_a(), &cwd) {
            Ok(sess) => AcpPaneState::Running(Box::new(sess)),
            Err(e) => AcpPaneState::Failed(e),
        };
        self.acp_panes.insert(pane_id, state);
        self.pane_status.insert(pane_id, AgentStatus::Working);
        self.focus_pane_in_active_tab(pane_id);
    }

    fn launch_team_grid(&mut self, req: TeamLaunchRequest) {
        let active_ws_idx = self.active_workspace_index;
        let (cwd, backend) = match self.workspaces.get(active_ws_idx) {
            Some(ws) => (ws.cwd.clone(), ws.backend.clone()),
            None => return,
        };

        let group_id = Uuid::new_v4();
        let coord_pid = PaneId::new();
        let worker_pids: Vec<PaneId> = req.worker_roles.iter().map(|_| PaneId::new()).collect();
        let layout = LayoutTree::create_team_grid(coord_pid, &worker_pids);

        self.pane_team_roles.insert(
            coord_pid,
            TeamRoleMeta {
                group_id,
                role_name: req.coordinator_role.clone(),
                is_coordinator: true,
            },
        );
        if let Ok(surf) = self.spawn_surface_for_backend(
            &backend,
            TerminalSize::new(80, 24),
            &cwd,
            HashMap::new(),
        ) {
            self.surfaces.insert(coord_pid, surf);
            self.osc_parsers.insert(coord_pid, OscParser::new());
            self.pane_status.insert(coord_pid, AgentStatus::Idle);
        }

        let adapter = AntigravityAdapter::new();
        let has_agy = adapter.detect_binary().is_some();
        let worker_cmd = AntigravityAdapter::shell_line_for_preset_with_options(
            &AgentLaunchPreset::NewSession,
            self.config.general.enforce_agy_sandbox,
        );

        for (&wpid, role) in worker_pids.iter().zip(req.worker_roles.iter()) {
            self.pane_team_roles.insert(
                wpid,
                TeamRoleMeta {
                    group_id,
                    role_name: role.clone(),
                    is_coordinator: false,
                },
            );
            if let Ok(mut surf) = self.spawn_surface_for_backend(
                &backend,
                TerminalSize::new(80, 24),
                &cwd,
                HashMap::new(),
            ) {
                if req.launch_agy_in_workers && has_agy {
                    let cmd = format!("{worker_cmd}\r");
                    let _ = surf.send_input(cmd.as_bytes());
                    self.pane_status.insert(wpid, AgentStatus::Working);
                } else {
                    self.pane_status.insert(wpid, AgentStatus::Idle);
                }
                self.surfaces.insert(wpid, surf);
                self.osc_parsers.insert(wpid, OscParser::new());
            }
        }

        if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
            let tab = Tab {
                id: agemax_core::layout::TabId::new(),
                title: format!("Team ({})", 1 + worker_pids.len()),
                layout,
                active_pane: coord_pid,
                zoomed_pane: None,
            };
            ws.tabs.push(tab);
            ws.active_tab_index = ws.tabs.len() - 1;
        }

        if req.use_acp_for_coordinator {
            self.open_acp_in_pane(coord_pid);
        } else if req.launch_agy_in_workers {
            self.launch_agy_in_pane(coord_pid);
        }
        self.save_session();
    }

    fn close_workspace(&mut self, idx: usize) -> bool {
        if self.workspaces.len() <= 1 || idx >= self.workspaces.len() {
            return false;
        }
        let ws = self.workspaces.remove(idx);
        for tab in ws.tabs {
            for p in tab.layout.panes() {
                self.remove_pane_resources(p);
            }
        }
        if self.active_workspace_index >= self.workspaces.len() {
            self.active_workspace_index = self.workspaces.len().saturating_sub(1);
        }
        self.save_session();
        true
    }

    fn create_new_tab(&mut self) {
        let active_ws_idx = self.active_workspace_index;
        let (cwd, backend) = match self.workspaces.get(active_ws_idx) {
            Some(ws) => (ws.cwd.clone(), ws.backend.clone()),
            None => return,
        };
        let new_pane = PaneId::new();
        if let Ok(surface) = self.spawn_surface_for_backend(
            &backend,
            TerminalSize::new(80, 24),
            &cwd,
            HashMap::new(),
        ) {
            self.surfaces.insert(new_pane, surface);
            self.osc_parsers.insert(new_pane, OscParser::new());
            self.pane_status.insert(new_pane, AgentStatus::Idle);
            if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                let tab_num = ws.tabs.len() + 1;
                ws.tabs
                    .push(Tab::new(format!("Terminal {}", tab_num), new_pane));
                ws.active_tab_index = ws.tabs.len() - 1;
            }
            self.save_session();
        }
    }

    fn focus_pane_in_active_tab(&mut self, pane_id: PaneId) {
        if let Some(ws) = self.active_workspace_mut() {
            if let Some(tab) = ws.active_tab_mut() {
                if tab.layout.panes().contains(&pane_id) {
                    tab.active_pane = pane_id;
                }
            }
        }
    }

    fn toggle_zoom_active(&mut self) {
        if let Some(ws) = self.active_workspace_mut() {
            if let Some(tab) = ws.active_tab_mut() {
                tab.toggle_zoom();
            }
        }
    }

    fn jump_to_oldest_unread(&mut self) {
        if let Some((ws_id, pane_id)) = self.notification_store.oldest_unread_pane() {
            if let Some(idx) = self.workspaces.iter().position(|w| w.id == ws_id) {
                self.active_workspace_index = idx;
                if let Some(ws) = self.workspaces.get_mut(idx) {
                    if let Some(tab) = ws.active_tab_mut() {
                        tab.active_pane = pane_id;
                    }
                }
                self.notification_store.mark_pane_read(pane_id);
            }
        }
    }

    fn switch_ui_mode(&mut self, mode: UiMode) {
        self.config.general.ui_mode = mode;
        self.config.general.show_toolbar = None;
        self.config.general.show_pane_headers = None;
        self.config.general.show_shortcut_hints = None;
        let _ = self.config.save_default();
    }

    fn launch_agy_in_pane(&mut self, pane_id: PaneId) {
        self.focus_pane_in_active_tab(pane_id);
        let detected = AntigravityAdapter::new().detect_binary();
        if detected.is_some() {
            let line = AntigravityAdapter::shell_line_for_preset_with_options(
                &AgentLaunchPreset::NewSession,
                self.config.general.enforce_agy_sandbox,
            );
            if let Some(surface) = self.surfaces.get_mut(&pane_id) {
                let cmd = format!("{line}\r");
                let _ = surface.send_input(cmd.as_bytes());
            }
        } else if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
            self.notification_store.notify(
                ws_id,
                pane_id,
                AgentStatus::Failed,
                "agy.exe not found. Run: irm https://antigravity.google/cli/install.ps1 | iex",
            );
            self.show_notifications_panel = true;
        }
    }

    fn active_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(self.active_workspace_index)
    }

    fn active_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.active_workspace_index)
    }

    fn active_pane_id(&self) -> Option<PaneId> {
        self.active_workspace().and_then(|ws| ws.active_pane_id())
    }

    fn find_workspace_for_pane(&self, pane_id: PaneId) -> Option<WorkspaceId> {
        for ws in &self.workspaces {
            for tab in &ws.tabs {
                if tab.layout.panes().contains(&pane_id) {
                    return Some(ws.id);
                }
            }
        }
        None
    }

    pub fn save_session(&mut self) {
        if self.pending_restore.is_some() {
            return;
        }
        // Best-effort conversation ID capture for active agent panes cleanly before writing snapshot
        let adapter = AntigravityAdapter::new();
        let mut captured = Vec::new();
        for ws in &self.workspaces {
            for tab in &ws.tabs {
                for &pid in &tab.layout.panes() {
                    if !self.agent_conversation_ids.contains_key(&pid)
                        && self.pane_status.get(&pid) != Some(&AgentStatus::Idle)
                    {
                        if let Some(cid) = adapter.capture_conversation_id(&ws.cwd) {
                            captured.push((pid, cid));
                        }
                    }
                }
            }
        }
        for (pid, cid) in captured {
            self.agent_conversation_ids.insert(pid, cid);
        }

        let ws_snapshots = self
            .workspaces
            .iter()
            .map(|ws| {
                let tab_snapshots = ws
                    .tabs
                    .iter()
                    .map(|tab| {
                        let layout_snap = LayoutSnapshot::from(&tab.layout);
                        let panes = tab
                            .layout
                            .panes()
                            .iter()
                            .map(|&pid| PaneSnapshot {
                                id: pid.0,
                                cwd: ws.cwd.clone(),
                                agent_conversation_id: self
                                    .agent_conversation_ids
                                    .get(&pid)
                                    .cloned(),
                                last_command: None,
                            })
                            .collect();
                        TabSnapshot {
                            name: tab.title.clone(),
                            active_pane_id: tab.active_pane.0,
                            layout: layout_snap,
                            panes,
                        }
                    })
                    .collect();
                WorkspaceSnapshot {
                    id: ws.id.0,
                    name: ws.name.clone(),
                    cwd: ws.cwd.clone(),
                    profile: ws.profile_label.clone(),
                    backend: ws.backend.clone(),
                    active_tab_index: ws.active_tab_index,
                    tabs: tab_snapshots,
                }
            })
            .collect();

        let snapshot = SessionSnapshot::new(self.active_workspace_index, ws_snapshots);
        let path = SessionSnapshot::default_path();
        let _ = snapshot.save_to_file(&path);
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::NewTab => self.create_new_tab(),
            PaletteAction::SplitRight => self.split_active_pane(SplitDirection::Horizontal),
            PaletteAction::SplitDown => self.split_active_pane(SplitDirection::Vertical),
            PaletteAction::ClosePane => self.close_active_pane(),
            PaletteAction::ToggleZoom => self.toggle_zoom_active(),
            PaletteAction::ToggleSidebar => {
                self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
            }
            PaletteAction::JumpUnread => self.jump_to_oldest_unread(),
            PaletteAction::ToggleNotifications => {
                self.show_notifications_panel = !self.show_notifications_panel;
            }
            PaletteAction::NewWorkspace => self.open_new_workspace_modal(),
            PaletteAction::LaunchAgy => {
                if let Some(pane_id) = self.active_pane_id() {
                    self.agent_launcher_modal
                        .open(pane_id, self.config.general.enforce_agy_sandbox);
                }
            }
            PaletteAction::OpenBrowser => {
                let _ = self.open_browser_in_pane(None, "http://localhost:3000");
            }
            PaletteAction::OpenAcpPanel => {
                if let Some(pane_id) = self.active_pane_id() {
                    self.open_acp_in_pane(pane_id);
                }
            }
            PaletteAction::LaunchTeamGrid => {
                self.team_launcher_modal.open();
            }
            PaletteAction::ToggleGridView => {
                self.show_grid_view = !self.show_grid_view;
                if self.show_grid_view {
                    self.show_welcome_view = false;
                }
            }
            PaletteAction::OpenBroadcast => {
                let targets = self.gather_broadcast_targets();
                self.broadcast_modal.open(&targets);
            }
            PaletteAction::OpenWelcome => {
                self.show_welcome_view = !self.show_welcome_view;
                if self.show_welcome_view {
                    self.show_grid_view = false;
                }
            }
            PaletteAction::ToggleUiMode => {
                let next = match self.config.general.ui_mode {
                    UiMode::Guided => UiMode::Minimal,
                    UiMode::Minimal => UiMode::Guided,
                };
                self.switch_ui_mode(next);
            }
            PaletteAction::OpenViewSettings => {
                self.settings_modal.open(&self.config);
            }
            PaletteAction::Custom(cmd) => {
                let target = self.active_pane_id();
                if let Some(id) = target {
                    if let Some(surface) = self.surfaces.get_mut(&id) {
                        let text = format!("{}\r", cmd.command);
                        let _ = surface.send_input(text.as_bytes());
                    }
                }
            }
        }
    }

    pub fn split_active_pane(&mut self, direction: SplitDirection) {
        let _ = self.split_active_pane_returning(direction);
    }

    pub fn split_active_pane_returning(&mut self, direction: SplitDirection) -> Option<PaneId> {
        let ws = self.active_workspace()?;
        let active_pane = ws.active_pane_id()?;
        let cwd = ws.cwd.clone();
        let backend = ws.backend.clone();

        let new_pane_id = PaneId::new();
        if let Ok(surface) = self.spawn_surface_for_backend(
            &backend,
            TerminalSize::new(80, 24),
            &cwd,
            HashMap::new(),
        ) {
            self.surfaces.insert(new_pane_id, surface);
            self.osc_parsers.insert(new_pane_id, OscParser::new());
            self.pane_status.insert(new_pane_id, AgentStatus::Idle);

            if let Some(ws) = self.active_workspace_mut() {
                if let Some(tab) = ws.active_tab_mut() {
                    tab.layout.split(active_pane, direction, new_pane_id);
                    tab.active_pane = new_pane_id;
                }
            }
            self.save_session();
            Some(new_pane_id)
        } else {
            None
        }
    }

    pub fn close_active_pane(&mut self) {
        let (active_pane, close_tab) = match self.active_workspace_mut() {
            Some(ws) => match ws.active_tab_mut() {
                Some(tab) => {
                    let pane = tab.active_pane;
                    match tab.layout.clone().remove(pane) {
                        Some(new_layout) => {
                            tab.layout = new_layout;
                            if let Some(next_pane) = tab.layout.panes().first() {
                                tab.active_pane = *next_pane;
                            }
                            (Some(pane), false)
                        }
                        None => (Some(pane), true),
                    }
                }
                None => (None, false),
            },
            None => (None, false),
        };

        if let Some(pane) = active_pane {
            self.remove_pane_resources(pane);
        }

        if close_tab {
            if let Some(ws) = self.active_workspace_mut() {
                if ws.tabs.len() > 1 {
                    ws.tabs.remove(ws.active_tab_index);
                    if ws.active_tab_index >= ws.tabs.len() {
                        ws.active_tab_index = ws.tabs.len().saturating_sub(1);
                    }
                }
            }
        }
        self.save_session();
    }

    pub fn navigate_focus(&mut self, dir: Direction, bounds: CoreRect) {
        if let Some(ws) = self.active_workspace_mut() {
            if let Some(tab) = ws.active_tab_mut() {
                if let Some(neighbor) = tab.layout.find_neighbor(tab.active_pane, dir, bounds) {
                    tab.active_pane = neighbor;
                }
            }
        }
    }

    fn handle_ipc_command(&mut self, cmd: IpcAppCommand) {
        match cmd {
            IpcAppCommand::WorkspaceList { reply } => {
                let list: Vec<serde_json::Value> = self
                    .workspaces
                    .iter()
                    .map(|ws| {
                        serde_json::json!({
                            "id": ws.id.0.to_string(),
                            "name": ws.name,
                            "cwd": ws.cwd.to_string_lossy(),
                            "active_tab": ws.active_tab_index,
                            "tab_count": ws.tabs.len(),
                        })
                    })
                    .collect();
                let _ = reply.send(serde_json::json!(list));
            }
            IpcAppCommand::WorkspaceCreate { name, cwd, reply } => {
                let dir = cwd
                    .map(PathBuf::from)
                    .unwrap_or_else(|| resolve_startup_directory(None, &self.config.general));
                let ws_name =
                    name.unwrap_or_else(|| format!("Workspace {}", self.workspaces.len() + 1));

                if let Err(err) =
                    validate_workspace_path(&dir, self.config.general.allowed_root_dir.as_deref())
                {
                    let _ = reply.send(serde_json::json!({ "error": err.to_string() }));
                } else if let Some(ws_id) =
                    self.create_workspace_in_dir(ws_name.clone(), dir, false)
                {
                    let _ = reply.send(serde_json::json!({
                        "id": ws_id.0.to_string(),
                        "name": ws_name
                    }));
                } else {
                    let _ = reply.send(serde_json::json!({ "error": "Failed to spawn surface" }));
                }
            }
            IpcAppCommand::WorkspaceClose { id, reply } => {
                if let Some(idx) = self.workspaces.iter().position(|w| w.id == id) {
                    if self.close_workspace(idx) {
                        let _ = reply.send(serde_json::json!({ "closed": id.0.to_string() }));
                    } else {
                        let _ = reply.send(serde_json::json!({
                            "error": "Cannot close only remaining workspace"
                        }));
                    }
                } else {
                    let _ = reply.send(serde_json::json!({ "error": "Workspace not found" }));
                }
            }
            IpcAppCommand::SplitPane { direction, reply } => {
                self.split_active_pane(direction);
                let _ = reply.send(serde_json::json!({
                    "active_pane": self.active_pane_id().map(|p| p.0.to_string())
                }));
            }
            IpcAppCommand::FocusPane { pane_id, reply } => {
                let mut found = false;
                for (w_idx, ws) in self.workspaces.iter_mut().enumerate() {
                    for tab in &mut ws.tabs {
                        if tab.layout.panes().contains(&pane_id) {
                            tab.active_pane = pane_id;
                            self.active_workspace_index = w_idx;
                            found = true;
                            break;
                        }
                    }
                    if found {
                        break;
                    }
                }
                let _ = reply.send(serde_json::json!({
                    "focused": pane_id.0.to_string(),
                    "found": found
                }));
            }
            IpcAppCommand::SendKeys {
                pane_id,
                keys,
                reply,
            } => {
                let target = pane_id.or_else(|| self.active_pane_id());
                let mut bytes_written = 0;
                if let Some(id) = target {
                    if let Some(surface) = self.surfaces.get_mut(&id) {
                        let _ = surface.send_input(keys.as_bytes());
                        bytes_written = keys.len();
                    }
                }
                let _ = reply.send(serde_json::json!({ "sent_bytes": bytes_written }));
            }
            IpcAppCommand::ReadScreen { pane_id, reply } => {
                let target = pane_id.or_else(|| self.active_pane_id());
                let mut screen_text = String::new();
                if let Some(id) = target {
                    if let Some(surface) = self.surfaces.get(&id) {
                        let rows = surface.rows();
                        let cols = surface.cols();
                        for r in 0..rows {
                            let mut line = String::new();
                            for c in 0..cols {
                                let cell = surface.get_cell(r, c);
                                if cell.c == '\0' {
                                    line.push(' ');
                                } else {
                                    line.push(cell.c);
                                }
                            }
                            screen_text.push_str(line.trim_end());
                            screen_text.push('\n');
                        }
                    }
                }
                let _ = reply.send(serde_json::json!(screen_text));
            }
            IpcAppCommand::Notify {
                title,
                message,
                status,
                reply,
            } => {
                let active_ws_id = self.active_workspace().map(|w| w.id).unwrap_or_default();
                let target_pane = self.active_pane_id().unwrap_or_default();
                let st = status.unwrap_or(AgentStatus::WaitingForInput);
                let full_msg = format!("{title}: {message}");

                self.notification_store
                    .notify(active_ws_id, target_pane, st, &full_msg);
                self.dispatch_attention_alerts(active_ws_id);
                let _ = reply.send(serde_json::json!({ "status": "notified" }));
            }
            IpcAppCommand::StatusSet {
                pane_id,
                status,
                message,
                reply,
            } => {
                let target = pane_id.or_else(|| self.active_pane_id());
                if let Some(id) = target {
                    self.pane_status.insert(id, status);
                    if let Some(msg) = message {
                        if let Some(ws_id) = self.find_workspace_for_pane(id) {
                            self.notification_store.notify(ws_id, id, status, msg);
                        }
                    }
                }
                let _ = reply.send(serde_json::json!({ "status": status.display_name() }));
            }
            IpcAppCommand::ResumeSession {
                pane_id,
                conversation_id,
                reply,
            } => {
                let target = pane_id.or_else(|| self.active_pane_id());
                let mut resumed = false;
                if let Some(id) = target {
                    let conv_id =
                        conversation_id.or_else(|| self.agent_conversation_ids.get(&id).cloned());
                    if let Some(cid) = conv_id {
                        if let Some(surface) = self.surfaces.get_mut(&id) {
                            let cmd = format!("agy --conversation {}\r", cid);
                            let _ = surface.send_input(cmd.as_bytes());
                            resumed = true;
                        }
                    }
                }
                let _ = reply.send(serde_json::json!({ "resumed": resumed }));
            }
            IpcAppCommand::ProfileList { reply } => {
                let pm = ProfileManager::new();
                let profiles: Vec<serde_json::Value> = pm
                    .list_profiles()
                    .into_iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "account_override": p.account_override,
                            "isolate_storage": p.isolate_storage,
                            "dir": p.profile_dir.to_string_lossy(),
                        })
                    })
                    .collect();
                let _ = reply.send(serde_json::json!(profiles));
            }
            IpcAppCommand::Activate { cwd, reply } => {
                let mut blocked_reason: Option<String> = None;
                if let Some(dir_str) = cwd {
                    let target_dir = PathBuf::from(&dir_str);
                    if target_dir.is_dir() {
                        match validate_workspace_path(
                            &target_dir,
                            self.config.general.allowed_root_dir.as_deref(),
                        ) {
                            Ok(safe_dir) => {
                                if let Some(pos) = self
                                    .workspaces
                                    .iter()
                                    .position(|ws| paths_match(&ws.cwd, &safe_dir))
                                {
                                    self.active_workspace_index = pos;
                                } else {
                                    let ws_name = safe_dir
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .filter(|n| !n.trim().is_empty())
                                        .unwrap_or_else(|| {
                                            format!("Workspace {}", self.workspaces.len() + 1)
                                        });
                                    let _ = self.create_workspace_in_dir(ws_name, safe_dir, false);
                                }
                            }
                            Err(err) => {
                                let msg = err.to_string();
                                blocked_reason = Some(msg.clone());
                                if let (Some(ws), Some(pane)) =
                                    (self.active_workspace(), self.active_pane_id())
                                {
                                    self.notification_store.notify(
                                        ws.id,
                                        pane,
                                        AgentStatus::Failed,
                                        msg,
                                    );
                                    self.show_notifications_panel = true;
                                }
                            }
                        }
                    }
                }
                self.pending_window_activate = true;
                let _ = reply.send(serde_json::json!({
                    "activated": true,
                    "active_workspace": self.active_workspace_index,
                    "blocked": blocked_reason,
                }));
            }
            IpcAppCommand::BrowserOpen {
                url,
                pane_id,
                reply,
            } => {
                if let Some(pid) = self.open_browser_in_pane(pane_id, &url) {
                    let final_url = self
                        .webview_panes
                        .get(&pid)
                        .map(|w| w.url.clone())
                        .unwrap_or(url);
                    let _ = reply.send(serde_json::json!({
                        "pane_id": pid.0.to_string(),
                        "url": final_url,
                    }));
                } else {
                    let _ =
                        reply.send(serde_json::json!({ "error": "Failed creating browser pane" }));
                }
            }
            IpcAppCommand::BrowserScreenshot { pane_id, reply } => {
                let target = pane_id
                    .filter(|p| self.webview_panes.contains_key(p))
                    .or_else(|| {
                        self.active_pane_id()
                            .filter(|p| self.webview_panes.contains_key(p))
                    })
                    .or_else(|| self.webview_panes.keys().next().copied());
                if let Some(pid) = target {
                    if let Some(wv) = self.webview_panes.get(&pid) {
                        wv.capture_screenshot(pid, reply);
                    }
                } else {
                    let _ = reply.send(serde_json::json!({ "error": "No browser pane open" }));
                }
            }
            IpcAppCommand::BrowserEval {
                pane_id,
                script,
                reply,
            } => {
                let target = pane_id
                    .filter(|p| self.webview_panes.contains_key(p))
                    .or_else(|| {
                        self.active_pane_id()
                            .filter(|p| self.webview_panes.contains_key(p))
                    })
                    .or_else(|| self.webview_panes.keys().next().copied());
                if let Some(pid) = target {
                    if let Some(wv) = self.webview_panes.get(&pid) {
                        wv.eval_js(&script, reply);
                    }
                } else {
                    let _ = reply.send(serde_json::json!({ "error": "No browser pane open" }));
                }
            }
        }
    }

    fn matches_keybinding(&self, spec: &str, key: Key, modifiers: &egui::Modifiers) -> bool {
        let Some(key_code) = egui_key_to_code(key) else {
            return false;
        };
        let Some(chord) = KeyChord::parse(spec) else {
            return false;
        };
        chord.matches(modifiers.ctrl, modifiers.shift, modifiers.alt, key_code)
    }

    fn handle_input(&mut self, ctx: &egui::Context, content_rect: CoreRect) {
        if self.is_modal_open() || self.context_menu.is_open {
            return;
        }

        let events = ctx.input(|i| i.events.clone());
        let has_paste_event = events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
        let focused_pane_id = self.active_pane_id();
        let focused_is_terminal = focused_pane_id
            .map(|id| {
                !self.show_welcome_view
                    && !self.show_grid_view
                    && !self.webview_panes.contains_key(&id)
                    && !self.acp_panes.contains_key(&id)
                    && !ctx.egui_wants_keyboard_input()
            })
            .unwrap_or(false);

        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if !focused_is_terminal {
                        continue;
                    }
                    if let Some(id) = focused_pane_id {
                        self.notification_store.mark_pane_read(id);
                        self.text_selection = None;
                        if self.pane_status.get(&id) == Some(&AgentStatus::WaitingForInput) {
                            self.pane_status.insert(id, AgentStatus::Working);
                        }
                        if let Some(surface) = self.surfaces.get_mut(&id) {
                            surface.scroll_to_bottom();
                            let _ = surface.send_input(text.as_bytes());
                        }
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Font Zoom Shortcuts (Ctrl+Plus/Equals, Ctrl+Minus, Ctrl+0)
                    if modifiers.ctrl && !modifiers.alt {
                        if matches!(key, Key::Plus | Key::Equals) {
                            self.adjust_font_zoom(1.0);
                            continue;
                        }
                        if key == Key::Minus {
                            self.adjust_font_zoom(-1.0);
                            continue;
                        }
                        if !modifiers.shift && key == Key::Num0 {
                            self.adjust_font_zoom(14.0 - self.font_size);
                            continue;
                        }
                    }

                    // Configurable global shortcuts (dynamic from KeybindingsConfig):
                    let kb = &self.config.keybindings;
                    if self.matches_keybinding(&kb.toggle_sidebar, key, &modifiers) {
                        self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
                        continue;
                    }
                    if self.matches_keybinding(&kb.command_palette, key, &modifiers) {
                        self.command_palette.open();
                        continue;
                    }
                    if self.matches_keybinding(&kb.new_tab, key, &modifiers) {
                        self.create_new_tab();
                        continue;
                    }
                    if self.matches_keybinding(&kb.split_horizontal, key, &modifiers) {
                        self.split_active_pane(SplitDirection::Horizontal);
                        continue;
                    }
                    if self.matches_keybinding(&kb.split_vertical, key, &modifiers) {
                        self.split_active_pane(SplitDirection::Vertical);
                        continue;
                    }
                    if self.matches_keybinding(&kb.close_pane, key, &modifiers) {
                        self.close_active_pane();
                        continue;
                    }
                    if self.matches_keybinding(&kb.zoom_pane, key, &modifiers) {
                        self.toggle_zoom_active();
                        continue;
                    }
                    if self.matches_keybinding(&kb.jump_unread, key, &modifiers) {
                        self.jump_to_oldest_unread();
                        continue;
                    }
                    if self.matches_keybinding(&kb.toggle_notifications, key, &modifiers) {
                        self.show_notifications_panel = !self.show_notifications_panel;
                        continue;
                    }

                    if modifiers.ctrl && modifiers.shift && key == Key::C {
                        self.copy_selection_to_clipboard(ctx, focused_pane_id);
                        continue;
                    }

                    if modifiers.ctrl && modifiers.shift && key == Key::V {
                        if !has_paste_event {
                            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                        }
                        continue;
                    }

                    // Directional navigation with Alt+Arrows
                    if modifiers.alt && !modifiers.ctrl && !modifiers.shift {
                        match key {
                            Key::ArrowLeft => {
                                self.navigate_focus(Direction::Left, content_rect);
                                continue;
                            }
                            Key::ArrowRight => {
                                self.navigate_focus(Direction::Right, content_rect);
                                continue;
                            }
                            Key::ArrowUp => {
                                self.navigate_focus(Direction::Up, content_rect);
                                continue;
                            }
                            Key::ArrowDown => {
                                self.navigate_focus(Direction::Down, content_rect);
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Forward keys to focused terminal surface
                    if !focused_is_terminal {
                        continue;
                    }
                    if let Some(id) = focused_pane_id {
                        self.notification_store.mark_pane_read(id);
                        self.text_selection = None;
                        if self.pane_status.get(&id) == Some(&AgentStatus::WaitingForInput) {
                            self.pane_status.insert(id, AgentStatus::Working);
                        }

                        if let Some(surface) = self.surfaces.get_mut(&id) {
                            surface.scroll_to_bottom();
                            if modifiers.ctrl && !modifiers.shift && !modifiers.alt {
                                match key {
                                    Key::C => {
                                        let _ = surface.send_input(b"\x03");
                                    }
                                    Key::D => {
                                        let _ = surface.send_input(b"\x04");
                                    }
                                    Key::Z => {
                                        let _ = surface.send_input(b"\x1a");
                                    }
                                    Key::L => {
                                        let _ = surface.send_input(b"\x0c");
                                    }
                                    Key::A => {
                                        let _ = surface.send_input(b"\x01");
                                    }
                                    Key::E => {
                                        let _ = surface.send_input(b"\x05");
                                    }
                                    Key::W => {
                                        let _ = surface.send_input(b"\x17");
                                    }
                                    Key::U => {
                                        let _ = surface.send_input(b"\x15");
                                    }
                                    Key::K => {
                                        let _ = surface.send_input(b"\x0b");
                                    }
                                    _ => {}
                                }
                            } else if !modifiers.ctrl && !modifiers.alt {
                                match key {
                                    Key::Enter => {
                                        let _ = surface.send_input(b"\r");
                                    }
                                    Key::Backspace => {
                                        let _ = surface.send_input(b"\x08");
                                    }
                                    Key::Tab => {
                                        let _ = surface.send_input(b"\t");
                                    }
                                    Key::Escape => {
                                        let _ = surface.send_input(b"\x1b");
                                    }
                                    Key::ArrowUp => {
                                        let _ = surface.send_input(b"\x1b[A");
                                    }
                                    Key::ArrowDown => {
                                        let _ = surface.send_input(b"\x1b[B");
                                    }
                                    Key::ArrowRight => {
                                        let _ = surface.send_input(b"\x1b[C");
                                    }
                                    Key::ArrowLeft => {
                                        let _ = surface.send_input(b"\x1b[D");
                                    }
                                    Key::Home => {
                                        let _ = surface.send_input(b"\x1b[H");
                                    }
                                    Key::End => {
                                        let _ = surface.send_input(b"\x1b[F");
                                    }
                                    Key::PageUp => {
                                        let _ = surface.send_input(b"\x1b[5~");
                                    }
                                    Key::PageDown => {
                                        let _ = surface.send_input(b"\x1b[6~");
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                egui::Event::Paste(text) => {
                    if !focused_is_terminal {
                        continue;
                    }
                    if let Some(id) = focused_pane_id {
                        self.notification_store.mark_pane_read(id);
                        self.text_selection = None;
                        if self.pane_status.get(&id) == Some(&AgentStatus::WaitingForInput) {
                            self.pane_status.insert(id, AgentStatus::Working);
                        }
                        if let Some(surface) = self.surfaces.get_mut(&id) {
                            surface.scroll_to_bottom();
                            let _ = surface.send_input(text.as_bytes());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn spawn_terminal_surface(
    size: TerminalSize,
    cwd: &Path,
    shell_pref: Option<&str>,
    env: HashMap<String, String>,
) -> Result<TerminalSurface, agemax_term::PtyError> {
    let config = agemax_term::LocalConPtyBackend.build_pty_config(cwd, shell_pref, size, env);
    TerminalSurface::new(size, config)
}

impl eframe::App for AgeMaxApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.first_frame_rendered {
            let cold_start_ms = self.start_time.elapsed().as_millis();
            tracing::info!(
                "Age Max First Render: {} ms (Budget < 300 ms)",
                cold_start_ms
            );
            self.first_frame_rendered = true;
        }

        self.preflight.poll();

        // 1. Check for configuration hot reload
        if let Some(watcher) = &self.config_watcher {
            if let Some(new_cfg) = watcher.try_recv() {
                self.font_size = new_cfg.general.font_size.clamp(9.0, 32.0);
                self.command_palette.sync_builtins(&new_cfg.keybindings);
                self.command_palette.set_custom_commands(&new_cfg.commands);
                self.config = new_cfg;
            }
        }

        // 2. Process pending IPC commands
        while let Ok(cmd) = self.ipc_rx.try_recv() {
            self.handle_ipc_command(cmd);
        }

        if self.pending_window_activate {
            self.pending_window_activate = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.notification_service.activate_window();
            ctx.request_repaint();
        }

        // 3. Process output, throttled git diff stats, ACP events, and per-pane activity telemetry
        let now_instant = Instant::now();
        let ws_git_targets: Vec<(WorkspaceId, PathBuf)> = self
            .workspaces
            .iter()
            .filter(|w| matches!(w.backend, WorkspaceBackend::Local))
            .map(|w| (w.id, w.cwd.clone()))
            .collect();
        self.git_stats_poller
            .poll_and_maybe_refresh(|| ws_git_targets, now_instant);

        let mut dirty = false;
        let mut acp_events_to_record: Vec<(PaneId, AcpEvent, AgentStatus)> = Vec::new();
        for (&pane_id, acp_state) in self.acp_panes.iter_mut() {
            if let AcpPaneState::Running(ref mut sess) = acp_state {
                let evs = sess.poll_events();
                if !evs.is_empty() {
                    dirty = true;
                    let next_status = sess.state.agent_status();
                    self.pane_status.insert(pane_id, next_status);
                    if let Some(tel) = self.pane_telemetry.get_mut(&pane_id) {
                        tel.heuristic_active = false;
                    }
                    for ev in evs {
                        acp_events_to_record.push((pane_id, ev, next_status));
                    }
                }
            }
        }
        for (pane_id, ev, next_status) in acp_events_to_record {
            match ev {
                AcpEvent::ToolCallStarted { tool, .. } => {
                    self.record_feed_for_pane(pane_id, "tool", format!("Running tool: {tool}"));
                }
                AcpEvent::ArtifactCreated { title, .. } => {
                    self.record_feed_for_pane(pane_id, "artifact", format!("Created {title}"));
                }
                AcpEvent::PromptCompleted => {
                    self.record_feed_for_pane(pane_id, "agent", "Finished ACP turn");
                    if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
                        self.notification_store.notify(
                            ws_id,
                            pane_id,
                            next_status,
                            "ACP agent turn completed",
                        );
                        self.dispatch_attention_alerts(ws_id);
                    }
                }
                AcpEvent::ProcessExited { message } => {
                    self.record_feed_for_pane(pane_id, "agent", message);
                }
                _ => {}
            }
        }

        let mut events_to_process = Vec::new();
        for (&pane_id, surface) in self.surfaces.iter_mut() {
            let parser = self.osc_parsers.entry(pane_id).or_default();
            let tel = self
                .pane_telemetry
                .entry(pane_id)
                .or_insert_with(|| PaneTelemetry::new(now_instant));
            let mut inferred_hint: Option<AgentStatus> = None;
            let got_bytes = surface.process_output_with(|chunk| {
                tel.record_bytes(chunk.len(), now_instant);
                if let Some(h) = infer_pty_tool_hint(chunk) {
                    inferred_hint = Some(h);
                }
                let evs = parser.parse(chunk);
                if !evs.is_empty() {
                    events_to_process.push((pane_id, evs));
                }
            });
            if got_bytes {
                dirty = true;
                tel.last_output_line = surface.last_non_empty_line();
                if let Some(hint) = inferred_hint {
                    let cur = self
                        .pane_status
                        .get(&pane_id)
                        .copied()
                        .unwrap_or(AgentStatus::Idle);
                    if cur == AgentStatus::Idle || tel.heuristic_active {
                        self.pane_status.insert(pane_id, hint);
                        tel.heuristic_active = true;
                    }
                }
            }
        }

        // Decay heuristic PTY burst back to Idle when silent and roll sparkline buckets forward
        let pane_ids_all: Vec<PaneId> = self.pane_status.keys().copied().collect();
        for pane_id in pane_ids_all {
            let status = self
                .pane_status
                .get(&pane_id)
                .copied()
                .unwrap_or(AgentStatus::Idle);
            let tel = self
                .pane_telemetry
                .entry(pane_id)
                .or_insert_with(|| PaneTelemetry::new(now_instant));
            tel.advance_time(now_instant);
            if tel.heuristic_active && status.is_working() {
                if let Some(last_at) = tel.last_pty_output_at {
                    if now_instant.saturating_duration_since(last_at).as_secs_f32() >= 2.2 {
                        tel.heuristic_active = false;
                        self.pane_status.insert(pane_id, AgentStatus::Idle);
                    }
                }
            }
            let updated_status = self
                .pane_status
                .get(&pane_id)
                .copied()
                .unwrap_or(AgentStatus::Idle);
            tel.sync_status(updated_status, now_instant);
        }

        for (pane_id, events) in events_to_process {
            for event in events {
                match event {
                    AgentOscEvent::StatusChange { status, message } => {
                        self.pane_status.insert(pane_id, status);
                        if let Some(tel) = self.pane_telemetry.get_mut(&pane_id) {
                            tel.heuristic_active = false;
                            tel.sync_status(status, now_instant);
                        }
                        if status.is_attention_needed() {
                            if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
                                let msg =
                                    message.unwrap_or_else(|| status.display_name().to_string());
                                self.notification_store.notify(ws_id, pane_id, status, &msg);
                                self.record_feed_for_pane(pane_id, "agent", msg);
                                self.dispatch_attention_alerts(ws_id);
                            }
                        } else if status == AgentStatus::RunningTool {
                            if let Some(ref msg) = message {
                                self.record_feed_for_pane(pane_id, "tool", msg.clone());
                            }
                        }
                    }
                    AgentOscEvent::Notification { title, message } => {
                        if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
                            let formatted = format!("{}: {}", title, message);
                            self.notification_store.notify(
                                ws_id,
                                pane_id,
                                AgentStatus::WaitingForInput,
                                &formatted,
                            );
                            self.record_feed_for_pane(pane_id, "agent", &formatted);
                            self.dispatch_attention_alerts(ws_id);
                        }
                    }
                    AgentOscEvent::Bell => {
                        if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
                            self.notification_store.notify(
                                ws_id,
                                pane_id,
                                AgentStatus::WaitingForInput,
                                "Terminal Bell",
                            );
                            self.dispatch_attention_alerts(ws_id);
                        }
                    }
                }
            }
        }

        if self.capture_done.load(std::sync::atomic::Ordering::Acquire) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if let Some(left) = self.capture_frames_left {
            if left == 0 {
                self.capture_frames_left = None;
                if let Some(out_path) = self.capture_out_path.take() {
                    let done_flag = std::sync::Arc::clone(&self.capture_done);
                    let repaint_ctx = ctx.clone();
                    ctx.request_screenshot(move |img| {
                        let [w, h] = img.size;
                        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
                        ppm.reserve(w * h * 3);
                        for px in &img.pixels {
                            ppm.push(px.r());
                            ppm.push(px.g());
                            ppm.push(px.b());
                        }
                        if let Some(parent) = out_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&out_path, ppm);
                        done_flag.store(true, std::sync::atomic::Ordering::Release);
                        repaint_ctx.request_repaint();
                    });
                }
            } else {
                self.capture_frames_left = Some(left - 1);
            }
        }

        if dirty || self.capture_frames_left.is_some() {
            ctx.request_repaint();
        } else if self.any_pane_animating {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(windows)]
        if self.host_hwnd.is_none() {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(wh) = frame.window_handle() {
                if let RawWindowHandle::Win32(h) = wh.as_raw() {
                    self.host_hwnd = Some(h.hwnd.get());
                }
            }
        }
        #[cfg(not(windows))]
        let _ = frame;

        let resolved_theme = self.config.theme.resolved_rgb();
        let tokens = ui_tokens::UiTokens::from_theme(&resolved_theme);
        if self.last_theme != Some(resolved_theme) {
            tokens.apply_visuals(ui.ctx());
            self.last_theme = Some(resolved_theme);
        }

        let mica_active = self
            .backdrop_ctrl
            .sync(self.host_hwnd, self.config.general.enable_mica_backdrop);
        let mut any_animating = false;
        let now_instant = Instant::now();
        let bg_rgb = resolved_theme.background;
        let window_fill = if mica_active && self.config.general.enable_mica_backdrop {
            Color32::from_rgba_unmultiplied(bg_rgb.0, bg_rgb.1, bg_rgb.2, 210)
        } else {
            color32_from_rgb(bg_rgb)
        };
        // 0. Full-width App Header Strip with Command Palette Search Pill (below native title bar)
        if self.config.general.effective_show_toolbar() {
            let header_act = egui::Panel::top("app_header_strip")
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    render_header_strip(ui, &self.config, &resolved_theme)
                })
                .inner;
            if let Some(ToolbarAction::OpenCommandPalette) = header_act {
                self.command_palette.open();
            }
        }

        // 0b. Bottom Status Bar (full window width when chrome visible)
        if self.config.general.effective_show_toolbar() {
            let active_ws = self.workspaces.get(self.active_workspace_index);
            let branch = active_ws.and_then(|w| w.git_branch.as_deref());
            let folder_name = active_ws
                .and_then(|w| w.cwd.file_name().map(|f| f.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "workspace".to_string());
            let running_agents = self
                .pane_status
                .values()
                .filter(|st| st.is_working())
                .count();
            let sb_act = egui::Panel::bottom("status_bar")
                .exact_size(ui_tokens::metrics::STATUS_BAR_HEIGHT)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    render_status_bar(
                        ui,
                        StatusBarProps {
                            git_branch: branch,
                            folder_name: &folder_name,
                            running_agents,
                            font_size: self.font_size,
                            command_palette_shortcut: &self.config.keybindings.command_palette,
                            resolved_theme: &resolved_theme,
                        },
                    )
                })
                .inner;
            if let Some(StatusBarAction::OpenCommandPalette) = sb_act {
                self.command_palette.open();
            }
        }

        // 1. Left Workspace Sidebar (full height between header and bottom status bar)
        let sidebar_width = if self.is_sidebar_collapsed {
            48.0
        } else {
            220.0
        };
        let sidebar_meta: Vec<SidebarWorkspaceMeta> = self
            .workspaces
            .iter()
            .map(|ws| {
                let unread_count = self.notification_store.unread_count_for_workspace(ws.id);
                let latest_snippet = self
                    .notification_store
                    .latest_unread_for_workspace(ws.id)
                    .map(|i| i.message.clone());
                let active_pid = ws.active_pane_id();
                let status = active_pid.and_then(|pid| self.pane_status.get(&pid).copied());
                let tel = active_pid.and_then(|pid| self.pane_telemetry.get(&pid));
                SidebarWorkspaceMeta {
                    unread_count,
                    latest_snippet,
                    last_output_line: tel.and_then(|t| t.last_output_line.clone()),
                    working_elapsed_secs: tel.and_then(|t| t.working_elapsed_secs(now_instant)),
                    git_stats: self.git_stats_poller.cached.get(&ws.id).copied(),
                    sparkline: tel.map(|t| t.sparkline).unwrap_or_default(),
                    status,
                }
            })
            .collect();

        let sidebar_action = egui::Panel::left("workspace_sidebar")
            .exact_size(sidebar_width)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                render_sidebar(
                    ui,
                    SidebarProps {
                        workspaces: &self.workspaces,
                        active_index: self.active_workspace_index,
                        is_collapsed: self.is_sidebar_collapsed,
                        meta: &sidebar_meta,
                        theme: &resolved_theme,
                    },
                )
            })
            .inner;

        if let Some(action) = sidebar_action {
            match action {
                SidebarAction::SelectWorkspace(idx) => {
                    self.active_workspace_index = idx;
                }
                SidebarAction::CreateWorkspace => self.open_new_workspace_modal(),
                SidebarAction::RenameWorkspace(idx) => {
                    if let Some(ws) = self.workspaces.get(idx) {
                        let current_name = ws.name.clone();
                        self.rename_modal
                            .open(RenameTarget::Workspace(idx), &current_name);
                    }
                }
                SidebarAction::CloseWorkspace(idx) => {
                    self.close_workspace(idx);
                }
                SidebarAction::ToggleCollapse => {
                    self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
                }
                SidebarAction::OpenViewSettings => {
                    self.settings_modal.open(&self.config);
                }
            }
        }

        // 2. Optional Right-Side Notification & Activity Feed Drawer (rendered before CentralPanel)
        if self.show_notifications_panel {
            let unread = self.notification_store.unread_items();
            let recent_feed = self.activity_feed.recent(40);
            let drawer_act = egui::Panel::right("right_drawer")
                .exact_size(272.0)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    render_right_drawer(
                        ui,
                        self.right_drawer_tab,
                        &unread,
                        &recent_feed,
                        &resolved_theme,
                    )
                })
                .inner;
            if let Some(drawer_act) = drawer_act {
                match drawer_act {
                    RightDrawerAction::SwitchTab(tab) => {
                        self.right_drawer_tab = tab;
                    }
                    RightDrawerAction::FocusPane(ws_id, pane_id) => {
                        if let Some(pos) = self.workspaces.iter().position(|w| w.id == ws_id) {
                            self.active_workspace_index = pos;
                            if let Some(ws) = self.workspaces.get_mut(pos) {
                                if let Some(tab) = ws.active_tab_mut() {
                                    tab.active_pane = pane_id;
                                }
                            }
                            self.notification_store.mark_pane_read(pane_id);
                            self.show_grid_view = false;
                            self.show_welcome_view = false;
                        }
                    }
                    RightDrawerAction::ClearFeed => {
                        self.activity_feed.clear();
                    }
                }
            }
        }

        // 3. Top Chrome Panel (Toolbar + Folder Lock Bar + Tab Strip + Optional Banners)
        let active_ws_idx = self.active_workspace_index;
        egui::Panel::top("top_chrome")
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(window_fill))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let total_unread: usize = self
                    .workspaces
                    .iter()
                    .map(|w| self.notification_store.unread_count_for_workspace(w.id))
                    .sum();
                if let Some(tb_action) = render_top_toolbar(
                    ui,
                    &self.config,
                    &resolved_theme,
                    self.is_sidebar_collapsed,
                    total_unread,
                ) {
                    match tb_action {
                        ToolbarAction::ToggleSidebar => {
                            self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
                        }
                        ToolbarAction::OpenNewWorkspace => self.open_new_workspace_modal(),
                        ToolbarAction::NewTab => self.create_new_tab(),
                        ToolbarAction::SplitRight => {
                            self.split_active_pane(SplitDirection::Horizontal);
                        }
                        ToolbarAction::SplitDown => {
                            self.split_active_pane(SplitDirection::Vertical);
                        }
                        ToolbarAction::LaunchAgy => {
                            if let Some(pane_id) = self.active_pane_id() {
                                self.agent_launcher_modal
                                    .open(pane_id, self.config.general.enforce_agy_sandbox);
                            }
                        }
                        ToolbarAction::OpenBrowserPane => {
                            let _ = self.open_browser_in_pane(None, "http://localhost:3000");
                        }
                        ToolbarAction::OpenAcpPanel => {
                            if let Some(pane_id) = self.active_pane_id() {
                                self.open_acp_in_pane(pane_id);
                            }
                        }
                        ToolbarAction::LaunchTeamGrid => {
                            self.team_launcher_modal.open();
                        }
                        ToolbarAction::ToggleGridView => {
                            self.show_grid_view = !self.show_grid_view;
                            if self.show_grid_view {
                                self.show_welcome_view = false;
                            }
                        }
                        ToolbarAction::OpenBroadcast => {
                            let targets = self.gather_broadcast_targets();
                            self.broadcast_modal.open(&targets);
                        }
                        ToolbarAction::OpenWelcome => {
                            self.show_welcome_view = !self.show_welcome_view;
                            if self.show_welcome_view {
                                self.show_grid_view = false;
                            }
                        }
                        ToolbarAction::OpenCommandPalette => self.command_palette.open(),
                        ToolbarAction::ToggleNotifications => {
                            self.show_notifications_panel = !self.show_notifications_panel;
                        }
                        ToolbarAction::OpenViewSettings => {
                            self.settings_modal.open(&self.config);
                        }
                        ToolbarAction::OpenSafetySettings => {
                            self.settings_modal
                                .open_tab(&self.config, SettingsTab::Safety);
                        }
                        ToolbarAction::SwitchUiMode(mode) => self.switch_ui_mode(mode),
                    }
                }

                if let Some(ws) = self.workspaces.get(self.active_workspace_index) {
                    if let Some(ToolbarAction::OpenSafetySettings) = render_workspace_folder_bar(
                        ui,
                        &ws.cwd,
                        &ws.backend,
                        self.config.general.allowed_root_dir.as_deref(),
                        self.config.general.enforce_agy_sandbox,
                        &resolved_theme,
                    ) {
                        self.settings_modal
                            .open_tab(&self.config, SettingsTab::Safety);
                    }
                }

                let show_hints = self.config.general.effective_show_shortcut_hints();
                let tab_action = if let Some(ws) = self.workspaces.get(active_ws_idx) {
                    render_tab_bar(
                        ui,
                        &ws.tabs,
                        ws.active_tab_index,
                        show_hints,
                        &resolved_theme,
                    )
                } else {
                    None
                };

                if let Some(action) = tab_action {
                    match action {
                        TabBarAction::Select(idx) => {
                            if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                                ws.active_tab_index = idx;
                            }
                        }
                        TabBarAction::Create => self.create_new_tab(),
                        TabBarAction::Rename(tab_index) => {
                            if let Some(ws) = self.workspaces.get(active_ws_idx) {
                                if let Some(tab) = ws.tabs.get(tab_index) {
                                    let current = tab.title.clone();
                                    self.rename_modal.open(
                                        RenameTarget::Tab {
                                            ws_index: active_ws_idx,
                                            tab_index,
                                        },
                                        &current,
                                    );
                                }
                            }
                        }
                        TabBarAction::Close(idx) => {
                            let closed_panes =
                                if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                                    if ws.tabs.len() > 1 {
                                        let closed_tab = ws.tabs.remove(idx);
                                        if ws.active_tab_index >= ws.tabs.len() {
                                            ws.active_tab_index = ws.tabs.len().saturating_sub(1);
                                        }
                                        Some(closed_tab.layout.panes())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                            if let Some(panes) = closed_panes {
                                for p in panes {
                                    self.remove_pane_resources(p);
                                }
                                self.save_session();
                            }
                        }
                    }
                }

                // Optional first-run agy.exe onboarding setup banner (REQ-075)
                if let Some(onb) = render_onboarding_banner(ui, &mut self.preflight, &self.config) {
                    match onb {
                        OnboardingAction::Recheck => {
                            self.preflight.trigger_check(
                                self.config.general.default_shell.clone(),
                                Some(ui.ctx().clone()),
                            );
                        }
                        OnboardingAction::HideForever => {
                            self.config.general.enable_onboarding_banner = false;
                            let _ = self.config.save_default();
                        }
                        OnboardingAction::Dismiss => {
                            self.preflight.session_dismissed = true;
                        }
                    }
                }

                // Optional non-blocking startup session restore prompt
                let mut restore_decision = None;
                if let Some(ref snap) = self.pending_restore {
                    restore_decision = render_session_restore_banner(ui, snap, &resolved_theme);
                }
                if let Some(decision) = restore_decision {
                    match decision {
                        SessionRestoreAction::RestoreWorkspacesOnly => {
                            if let Some(snap) = self.pending_restore.take() {
                                self.restore_session_snapshot(snap, false);
                            }
                        }
                        SessionRestoreAction::RestoreAndResumeAgents => {
                            if let Some(snap) = self.pending_restore.take() {
                                self.restore_session_snapshot(snap, true);
                            }
                        }
                        SessionRestoreAction::Dismiss => {
                            self.pending_restore = None;
                        }
                    }
                }
            });

        // 4. Main Central Viewport (Welcome / Agents Grid / Split Terminal & ACP & WebView Canvas)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(window_fill)
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ui, |ui| {
                let avail_rect = ui.available_rect_before_wrap();
                let content_rect = CoreRect::from_min_size(
                    Point::new(avail_rect.min.x, avail_rect.min.y),
                    avail_rect.width(),
                    avail_rect.height(),
                );

                // Handle keyboard input routed to focused pane
                self.handle_input(ui.ctx(), content_rect);

                    // Handle global Ctrl+Wheel font zoom + hover mouse wheel scrollback / drag splitters
                    let raw_scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
                    let mods_now = ui.input(|i| i.modifiers);
                    let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
                    let primary_down = ui.input(|i| i.pointer.primary_down());
                    let primary_released = ui.input(|i| i.pointer.primary_released());
                    let secondary_clicked = ui.input(|i| i.pointer.secondary_clicked());
                    let mut scrollback_target: Option<(PaneId, i32)> = None;

                    let mut visible_webviews: HashSet<PaneId> = HashSet::new();
                    let mut pending_pane_action: Option<PaneHeaderAction> = None;
                    let mut pending_exit_action: Option<ExitBannerAction> = None;
                    let mut pending_browser_action: Option<BrowserPaneAction> = None;
                    let mut pending_acp_action: Option<AcpPaneAction> = None;
                    let mut clicked_focus_pane: Option<PaneId> = None;
                    let mut url_to_open: Option<String> = None;

                    if self.show_welcome_view {
                        let recent_snap =
                            SessionSnapshot::load_from_file(&SessionSnapshot::default_path()).ok();
                        let active_cwd = self
                            .active_workspace()
                            .map(|w| w.cwd.clone())
                            .unwrap_or_else(|| PathBuf::from("."));
                        if let Some(w_act) = render_welcome_view(
                            ui,
                            recent_snap.as_ref(),
                            &active_cwd,
                            &resolved_theme,
                        ) {
                            match w_act {
                                WelcomeAction::LaunchNewAgent => {
                                    self.launch_quick_preset_in_active_pane(
                                        AgentLaunchPreset::NewSession,
                                    );
                                }
                                WelcomeAction::LaunchSafePlanAgent => {
                                    self.launch_quick_preset_in_active_pane(
                                        AgentLaunchPreset::SafePlanMode,
                                    );
                                }
                                WelcomeAction::LaunchTeamGrid => {
                                    self.show_welcome_view = false;
                                    self.team_launcher_modal.open();
                                }
                                WelcomeAction::OpenNewWorkspaceModal => {
                                    self.show_welcome_view = false;
                                    self.open_new_workspace_modal();
                                }
                                WelcomeAction::RestoreRecentWorkspace(path) => {
                                    if let Some(pos) = self
                                        .workspaces
                                        .iter()
                                        .position(|w| paths_match(&w.cwd, &path))
                                    {
                                        self.active_workspace_index = pos;
                                        self.show_welcome_view = false;
                                    } else {
                                        let name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Workspace")
                                            .to_string();
                                        let _ = self.create_workspace_in_dir(name, path, false);
                                        self.show_welcome_view = false;
                                    }
                                }
                                WelcomeAction::Dismiss => {
                                    self.show_welcome_view = false;
                                }
                            }
                        }
                    } else if self.show_grid_view {
                        let mut grid_cards = Vec::new();
                        for (w_idx, ws) in self.workspaces.iter().enumerate() {
                            for (t_idx, tab) in ws.tabs.iter().enumerate() {
                                for pid in tab.layout.panes() {
                                    let status = self
                                        .pane_status
                                        .get(&pid)
                                        .copied()
                                        .unwrap_or(AgentStatus::Idle);
                                    let is_focused = w_idx == active_ws_idx
                                        && t_idx == ws.active_tab_index
                                        && pid == tab.active_pane;
                                    let tel = self.pane_telemetry.get(&pid);
                                    let kind_label = if self.acp_panes.contains_key(&pid) {
                                        "ACP Panel"
                                    } else if self.webview_panes.contains_key(&pid) {
                                        "Browser"
                                    } else {
                                        "Terminal"
                                    };
                                    let last_line = if let Some(AcpPaneState::Running(s)) =
                                        self.acp_panes.get(&pid)
                                    {
                                        s.state
                                            .latest_message
                                            .lines()
                                            .rev()
                                            .find(|l| !l.trim().is_empty())
                                            .map(|l| l.trim().to_string())
                                            .or_else(|| s.state.latest_status.clone())
                                    } else {
                                        tel.and_then(|t| t.last_output_line.clone())
                                    };
                                    grid_cards.push(GridPaneCard {
                                        workspace_index: w_idx,
                                        workspace_name: ws.name.clone(),
                                        tab_index: t_idx,
                                        tab_title: tab.title.clone(),
                                        pane_id: pid,
                                        status,
                                        is_focused,
                                        git_branch: ws.git_branch.clone(),
                                        git_stats: self
                                            .git_stats_poller
                                            .cached
                                            .get(&ws.id)
                                            .copied(),
                                        last_line,
                                        elapsed_secs: tel
                                            .and_then(|t| t.working_elapsed_secs(now_instant)),
                                        sparkline: tel.map(|t| t.sparkline).unwrap_or_default(),
                                        kind_label,
                                    });
                                }
                            }
                        }

                        if let Some(g_act) =
                            render_agents_grid(ui, &grid_cards, &resolved_theme)
                        {
                            match g_act {
                                GridViewAction::FocusPane {
                                    workspace_index,
                                    tab_index,
                                    pane_id,
                                } => {
                                    self.active_workspace_index = workspace_index;
                                    if let Some(ws) = self.workspaces.get_mut(workspace_index) {
                                        ws.active_tab_index =
                                            tab_index.min(ws.tabs.len().saturating_sub(1));
                                        if let Some(tab) = ws.active_tab_mut() {
                                            tab.active_pane = pane_id;
                                        }
                                    }
                                    self.notification_store.mark_pane_read(pane_id);
                                    self.show_grid_view = false;
                                }
                                GridViewAction::ExitGridView => {
                                    self.show_grid_view = false;
                                }
                            }
                        }
                    } else {
                        ui.allocate_rect(avail_rect, egui::Sense::hover());
                        if !self.is_modal_open() && !self.context_menu.is_open && raw_scroll_y.abs() > 0.1
                        {
                        if mods_now.ctrl {
                            let step = if raw_scroll_y > 0.0 { 1.0 } else { -1.0 };
                            self.adjust_font_zoom(step);
                        } else if let Some(pos) = pointer_pos {
                            if let Some(ws) = self.workspaces.get(active_ws_idx) {
                                if let Some(tab) = ws.active_tab() {
                                    let rects = if let Some(z) = tab.zoomed_pane {
                                        let mut m = HashMap::new();
                                        m.insert(z, content_rect);
                                        m
                                    } else {
                                        tab.layout.compute_rects(content_rect)
                                    };
                                    for (pid, r) in rects {
                                        let egui_r = Rect::from_min_size(
                                            Pos2::new(r.min.x, r.min.y),
                                            Vec2::new(r.width(), r.height()),
                                        );
                                        if egui_r.contains(pos) {
                                            let lines = (raw_scroll_y / 18.0).round() as i32;
                                            let delta = if lines == 0 {
                                                if raw_scroll_y > 0.0 { 1 } else { -1 }
                                            } else {
                                                lines
                                            };
                                            scrollback_target = Some((pid, delta));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some((pid, delta)) = scrollback_target {
                        if let Some(surface) = self.surfaces.get_mut(&pid) {
                            surface.scroll_delta(delta);
                        }
                    }

                    // Handle interactive split divider drag-to-resize (REQ-071)
                    let mut divider_dragging_or_hovered = false;
                    if !self.is_modal_open() && !self.context_menu.is_open {
                        if let Some(drag) = self.active_split_drag.clone() {
                            divider_dragging_or_hovered = true;
                            ui.ctx().set_cursor_icon(match drag.direction {
                                SplitDirection::Horizontal => egui::CursorIcon::ResizeHorizontal,
                                SplitDirection::Vertical => egui::CursorIcon::ResizeVertical,
                            });
                            if primary_down {
                                if let Some(pos) = pointer_pos {
                                    let ratio = match drag.direction {
                                        SplitDirection::Horizontal => {
                                            (pos.x - drag.container.min.x)
                                                / drag.container.width().max(1.0)
                                        }
                                        SplitDirection::Vertical => {
                                            (pos.y - drag.container.min.y)
                                                / drag.container.height().max(1.0)
                                        }
                                    };
                                    if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                                        if let Some(tab) = ws.active_tab_mut() {
                                            tab.layout.set_split_ratio_at_path(&drag.path, ratio);
                                        }
                                    }
                                }
                            }
                            if primary_released || !primary_down {
                                self.active_split_drag = None;
                                self.save_session();
                            }
                        } else if let Some(pos) = pointer_pos {
                            if let Some(ws) = self.workspaces.get(active_ws_idx) {
                                if let Some(tab) = ws.active_tab() {
                                    if tab.zoomed_pane.is_none() {
                                        for div in tab.layout.split_dividers(content_rect) {
                                            let hit_rect = match div.direction {
                                                SplitDirection::Horizontal => Rect::from_min_max(
                                                    Pos2::new(div.min.x - 4.0, div.min.y),
                                                    Pos2::new(div.max.x + 4.0, div.max.y),
                                                ),
                                                SplitDirection::Vertical => Rect::from_min_max(
                                                    Pos2::new(div.min.x, div.min.y - 4.0),
                                                    Pos2::new(div.max.x, div.max.y + 4.0),
                                                ),
                                            };
                                            if hit_rect.contains(pos) {
                                                divider_dragging_or_hovered = true;
                                                ui.ctx().set_cursor_icon(match div.direction {
                                                    SplitDirection::Horizontal => {
                                                        egui::CursorIcon::ResizeHorizontal
                                                    }
                                                    SplitDirection::Vertical => {
                                                        egui::CursorIcon::ResizeVertical
                                                    }
                                                });
                                                ui.painter().line_segment(
                                                    [
                                                        Pos2::new(div.min.x, div.min.y),
                                                        Pos2::new(div.max.x, div.max.y),
                                                    ],
                                                    Stroke::new(
                                                        2.5,
                                                        color32_from_rgb(
                                                            resolved_theme.active_border,
                                                        ),
                                                    ),
                                                );
                                                if primary_pressed {
                                                    self.active_split_drag = Some(ActiveSplitDrag {
                                                        path: div.path,
                                                        direction: div.direction,
                                                        container: Rect::from_min_max(
                                                            Pos2::new(
                                                                div.container.min.x,
                                                                div.container.min.y,
                                                            ),
                                                            Pos2::new(
                                                                div.container.max.x,
                                                                div.container.max.y,
                                                            ),
                                                        ),
                                                    });
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 3. Render panes inside the tab layout
                    let show_pane_headers = self.config.general.effective_show_pane_headers();
                    let show_hints = self.config.general.effective_show_shortcut_hints();
                    let modal_or_menu_open = self.is_modal_open() || self.context_menu.is_open;
                    let host_hwnd = self.host_hwnd;
                    let pane_bg_color = if mica_active && self.config.general.enable_mica_backdrop {
                        Color32::from_rgba_unmultiplied(bg_rgb.0, bg_rgb.1, bg_rgb.2, 218)
                    } else {
                        color32_from_rgb(resolved_theme.background)
                    };
                    let default_fg_color = color32_from_rgb(resolved_theme.foreground);
                    let sel_fill = if resolved_theme.is_light_background() {
                        Color32::from_rgba_unmultiplied(60, 120, 225, 80)
                    } else {
                        Color32::from_rgba_unmultiplied(70, 125, 225, 110)
                    };

                    if let Some(ws) = self.workspaces.get(active_ws_idx) {
                        if let Some(tab) = ws.active_tab() {
                            let focused_pane = tab.active_pane;
                            let is_zoomed = tab.zoomed_pane.is_some();
                            let pane_rects = if let Some(zoomed) = tab.zoomed_pane {
                                let mut m = HashMap::new();
                                m.insert(zoomed, content_rect);
                                m
                            } else {
                                tab.layout.compute_rects(content_rect)
                            };

                            let font_id = FontId::monospace(self.font_size);
                            let sample = ui.painter().layout_no_wrap(
                                "M".to_string(),
                                font_id.clone(),
                                default_fg_color,
                            );
                            self.char_width = sample.size().x.max(4.0);
                            self.char_height = sample.size().y.max(8.0);

                            for (pane_id, rect) in pane_rects {
                                let is_focused = pane_id == focused_pane;
                                let target_egui_rect = Rect::from_min_size(
                                    Pos2::new(rect.min.x, rect.min.y),
                                    Vec2::new(rect.width(), rect.height()),
                                );
                                let rect_anim = self.rect_animator.step(
                                    pane_id,
                                    target_egui_rect,
                                    self.active_split_drag.is_some(),
                                    now_instant,
                                );
                                any_animating |= rect_anim.animating;
                                let egui_rect = rect_anim.visual_rect;

                                // Pane background and smooth state-colored attention glow (`REQ-084`)
                                ui.painter().rect_filled(egui_rect, 2.0, pane_bg_color);
                                let pane_status = self
                                    .pane_status
                                    .get(&pane_id)
                                    .copied()
                                    .unwrap_or(AgentStatus::Idle);
                                let team_meta = self.pane_team_roles.get(&pane_id).cloned();
                                let team_color = team_meta.as_ref().map(|m| {
                                    let c = team_badge_color(m.is_coordinator);
                                    (c.r(), c.g(), c.b())
                                });

                                let border_visual = self.visual_animator.compute(
                                    pane_id,
                                    pane_status,
                                    is_focused,
                                    team_color,
                                    &resolved_theme,
                                    now_instant,
                                );
                                any_animating |= border_visual.animating;

                                if let Some(glow_col) = border_visual.outer_glow_color {
                                    ui.painter().rect_stroke(
                                        egui_rect.shrink(1.0),
                                        3.0,
                                        Stroke::new(border_visual.outer_glow_width, glow_col),
                                        StrokeKind::Inside,
                                    );
                                }

                                ui.painter().rect_stroke(
                                    egui_rect,
                                    2.0,
                                    Stroke::new(
                                        border_visual.border_width,
                                        border_visual.border_color,
                                    ),
                                    StrokeKind::Inside,
                                );

                                if let Some(focus_col) = border_visual.focus_inner_color {
                                    ui.painter().rect_stroke(
                                        egui_rect.shrink(2.0),
                                        1.5,
                                        Stroke::new(1.0, focus_col),
                                        StrokeKind::Inside,
                                    );
                                }

                                // Optional Guided mode slim 26px per-pane header bar
                                let header_h = if show_pane_headers { 26.0 } else { 0.0 };
                                if show_pane_headers && egui_rect.height() > header_h + 40.0 {
                                    let header_rect = Rect::from_min_max(
                                        egui_rect.min,
                                        Pos2::new(egui_rect.max.x, egui_rect.min.y + header_h),
                                    );
                                    let team_role_prop = team_meta
                                        .as_ref()
                                        .map(|m| (m.role_name.as_str(), m.is_coordinator));
                                    if let Some(act) = render_pane_header(
                                        ui,
                                        PaneHeaderProps {
                                            header_rect,
                                            pane_id,
                                            status: pane_status,
                                            is_focused,
                                            is_zoomed,
                                            show_hints,
                                            show_mascot: self.config.general.show_agent_mascots,
                                            resolved_theme: &resolved_theme,
                                            keybindings: &self.config.keybindings,
                                            team_role: team_role_prop,
                                        },
                                    ) {
                                        pending_pane_action = Some(act);
                                    }
                                }

                                // Calculate body region below optional header
                                let pad = 4.0;
                                let term_top = egui_rect.min.y + header_h;
                                let term_rect = Rect::from_min_max(
                                    Pos2::new(egui_rect.min.x, term_top),
                                    egui_rect.max,
                                );
                                let term_size = compute_terminal_grid_size(
                                    target_egui_rect.width(),
                                    target_egui_rect.height(),
                                    header_h,
                                    pad,
                                    self.char_width,
                                    self.char_height,
                                );
                                let origin = Pos2::new(egui_rect.min.x + pad, term_top + pad);

                                // Handle mouse focus and right-click menu trigger
                                if !modal_or_menu_open && !divider_dragging_or_hovered {
                                    if let Some(pos) = pointer_pos {
                                        if egui_rect.contains(pos)
                                            && (primary_pressed || secondary_clicked)
                                        {
                                            clicked_focus_pane = Some(pane_id);
                                        }
                                        if self.surfaces.contains_key(&pane_id)
                                            && !self.webview_panes.contains_key(&pane_id)
                                            && !self.acp_panes.contains_key(&pane_id)
                                            && term_rect.contains(pos)
                                            && secondary_clicked
                                        {
                                            self.context_menu = ContextMenuState {
                                                is_open: true,
                                                pane_id: Some(pane_id),
                                                pos,
                                            };
                                        }
                                    }
                                }

                                if let Some(wv) = self.webview_panes.get_mut(&pane_id) {
                                    let allow_overlay =
                                        !modal_or_menu_open && rect_anim.allow_backend_resize;
                                    if allow_overlay {
                                        visible_webviews.insert(pane_id);
                                    }
                                    let ctx_clone = ui.ctx().clone();
                                    if let Some(b_act) = render_browser_pane(
                                        ui,
                                        &ctx_clone,
                                        pane_id,
                                        term_rect,
                                        wv,
                                        allow_overlay,
                                        host_hwnd,
                                    ) {
                                        pending_browser_action = Some(b_act);
                                    }
                                    continue;
                                }

                                if let Some(acp_state) = self.acp_panes.get_mut(&pane_id) {
                                    if let Some(a_act) =
                                        render_acp_pane(ui, pane_id, term_rect, acp_state)
                                    {
                                        pending_acp_action = Some(a_act);
                                    }
                                    continue;
                                }

                                let mut jump_to_bottom_clicked = false;

                                if let Some(surface) = self.surfaces.get_mut(&pane_id) {
                                    if rect_anim.allow_backend_resize {
                                        let _ = surface.resize(term_size);
                                    }

                                    let cur_rows = surface.rows();
                                    let cur_cols = surface.cols();
                                    let scroll_offset = surface.display_offset();
                                    let surface_exited = surface.has_exited();

                                    // Pointer hover URL check & click-drag text selection inside this pane
                                    if !modal_or_menu_open && !divider_dragging_or_hovered {
                                        if let Some(pos) = pointer_pos {
                                            if mods_now.ctrl && term_rect.contains(pos) {
                                                if let Some((vr, vc)) = viewport_cell_at_pos(
                                                    pos,
                                                    origin,
                                                    self.char_width,
                                                    self.char_height,
                                                    cur_rows,
                                                    cur_cols,
                                                ) {
                                                    if let Some(url) = surface.url_at(vr, vc) {
                                                        ui.ctx().set_cursor_icon(
                                                            egui::CursorIcon::PointingHand,
                                                        );
                                                        if primary_pressed {
                                                            url_to_open = Some(url);
                                                        }
                                                    }
                                                }
                                            } else if primary_pressed
                                                && term_rect.contains(pos)
                                                && url_to_open.is_none()
                                            {
                                                if let Some((vr, vc)) = viewport_cell_at_pos(
                                                    pos,
                                                    origin,
                                                    self.char_width,
                                                    self.char_height,
                                                    cur_rows,
                                                    cur_cols,
                                                ) {
                                                    let pt = GridPoint {
                                                        line: surface.viewport_to_grid_line(vr),
                                                        col: vc,
                                                    };
                                                    self.text_selection = Some(TextSelection {
                                                        pane_id,
                                                        anchor: pt,
                                                        end: pt,
                                                        is_dragging: true,
                                                    });
                                                }
                                            } else if primary_down {
                                                if let Some(ref mut sel) = self.text_selection {
                                                    if sel.pane_id == pane_id && sel.is_dragging {
                                                        let clamped = Pos2::new(
                                                            pos.x.clamp(
                                                                origin.x,
                                                                origin.x
                                                                    + (cur_cols.saturating_sub(1)
                                                                        as f32
                                                                        * self.char_width),
                                                            ),
                                                            pos.y.clamp(
                                                                origin.y,
                                                                origin.y
                                                                    + (cur_rows.saturating_sub(1)
                                                                        as f32
                                                                        * self.char_height),
                                                            ),
                                                        );
                                                        if let Some((vr, vc)) = viewport_cell_at_pos(
                                                            clamped,
                                                            origin,
                                                            self.char_width,
                                                            self.char_height,
                                                            cur_rows,
                                                            cur_cols,
                                                        ) {
                                                            sel.end = GridPoint {
                                                                line: surface
                                                                    .viewport_to_grid_line(vr),
                                                                col: vc,
                                                            };
                                                        }
                                                    }
                                                }
                                            }
                                            if primary_released || !primary_down {
                                                if let Some(ref mut sel) = self.text_selection {
                                                    if sel.pane_id == pane_id {
                                                        sel.is_dragging = false;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    let painter = ui.painter().with_clip_rect(term_rect);
                                    let active_sel = self.text_selection;

                                    for r in 0..cur_rows {
                                        let y = origin.y + (r as f32 * self.char_height);
                                        if y >= term_rect.max.y {
                                            break;
                                        }
                                        let abs_line = surface.viewport_to_grid_line(r);
                                        let mut span_text = String::new();
                                        let mut span_start_c = 0;
                                        let mut span_fg = default_fg_color;

                                        for c in 0..cur_cols {
                                            let cell = surface.get_cell_for_theme(
                                                r,
                                                c,
                                                resolved_theme.background,
                                                resolved_theme.foreground,
                                            );
                                            let x = origin.x + (c as f32 * self.char_width);
                                            let cell_rect = Rect::from_min_size(
                                                Pos2::new(x, y),
                                                Vec2::new(self.char_width, self.char_height),
                                            );

                                            if let Some(bg) = cell.bg {
                                                let matches_theme_bg =
                                                    (bg.r, bg.g, bg.b) == resolved_theme.background;
                                                let dark_black_blend =
                                                    !resolved_theme.is_light_background()
                                                        && bg.r < 5
                                                        && bg.g < 5
                                                        && bg.b < 5;
                                                if !matches_theme_bg && !dark_black_blend {
                                                    painter.rect_filled(
                                                        cell_rect,
                                                        0.0,
                                                        Color32::from_rgb(bg.r, bg.g, bg.b),
                                                    );
                                                }
                                            }

                                            if let Some(sel) = active_sel {
                                                if sel.contains_cell(pane_id, abs_line, c) {
                                                    painter.rect_filled(cell_rect, 0.0, sel_fill);
                                                }
                                            }

                                            let char_fg = if let Some(fg) = cell.fg {
                                                Color32::from_rgb(fg.r, fg.g, fg.b)
                                            } else {
                                                default_fg_color
                                            };

                                            if cell.c == ' ' || cell.c == '\0' {
                                                if !span_text.is_empty() {
                                                    let span_x = origin.x
                                                        + (span_start_c as f32 * self.char_width);
                                                    painter.text(
                                                        Pos2::new(span_x, y),
                                                        egui::Align2::LEFT_TOP,
                                                        &span_text,
                                                        font_id.clone(),
                                                        span_fg,
                                                    );
                                                    span_text.clear();
                                                }
                                                span_start_c = c + 1;
                                            } else if span_text.is_empty() {
                                                span_text.push(cell.c);
                                                span_start_c = c;
                                                span_fg = char_fg;
                                            } else if span_fg == char_fg {
                                                span_text.push(cell.c);
                                            } else {
                                                let span_x = origin.x
                                                    + (span_start_c as f32 * self.char_width);
                                                painter.text(
                                                    Pos2::new(span_x, y),
                                                    egui::Align2::LEFT_TOP,
                                                    &span_text,
                                                    font_id.clone(),
                                                    span_fg,
                                                );
                                                span_text.clear();
                                                span_text.push(cell.c);
                                                span_start_c = c;
                                                span_fg = char_fg;
                                            }
                                        }

                                        if !span_text.is_empty() {
                                            let span_x =
                                                origin.x + (span_start_c as f32 * self.char_width);
                                            painter.text(
                                                Pos2::new(span_x, y),
                                                egui::Align2::LEFT_TOP,
                                                &span_text,
                                                font_id.clone(),
                                                span_fg,
                                            );
                                        }
                                    }

                                    // Draw cursor if pane is focused and at live bottom of scrollback
                                    if is_focused && scroll_offset == 0 && !surface_exited {
                                        let (cursor_row, cursor_col) = surface.cursor_pos();
                                        if cursor_row < cur_rows && cursor_col < cur_cols {
                                            let cur_x =
                                                origin.x + (cursor_col as f32 * self.char_width);
                                            let cur_y =
                                                origin.y + (cursor_row as f32 * self.char_height);
                                            let cursor_rect = Rect::from_min_size(
                                                Pos2::new(cur_x, cur_y),
                                                Vec2::new(self.char_width, self.char_height),
                                            );
                                            let (cr, cg, cb) = resolved_theme.cursor;
                                            painter.rect_filled(
                                                cursor_rect,
                                                1.0,
                                                Color32::from_rgba_unmultiplied(cr, cg, cb, 165),
                                            );
                                        }
                                    }

                                    // Subtle "Scrolled up" pill in bottom-right corner
                                    if scroll_offset > 0 && egui_rect.width() > 160.0 {
                                        let pill_rect = Rect::from_min_size(
                                            Pos2::new(
                                                egui_rect.max.x - 148.0,
                                                egui_rect.max.y - 26.0,
                                            ),
                                            Vec2::new(140.0, 20.0),
                                        );
                                        ui.scope_builder(
                                            egui::UiBuilder::new()
                                                .id_salt(("scroll_pill", pane_id.0))
                                                .max_rect(pill_rect),
                                            |ui| {
                                                if ui
                                                    .small_button(icons::icon_small(
                                                        icons::UiIcon::ArrowDown,
                                                        &format!("Latest (+{})", scroll_offset),
                                                    ))
                                                    .clicked()
                                                {
                                                    jump_to_bottom_clicked = true;
                                                }
                                            },
                                        );
                                    }

                                    // Inline exit recovery bar if child shell/agent exited (REQ-074)
                                    if surface_exited {
                                        let has_conv =
                                            self.agent_conversation_ids.contains_key(&pane_id);
                                        if let Some(act) = render_pane_exit_banner(
                                            ui, egui_rect, pane_id, has_conv,
                                        ) {
                                            pending_exit_action = Some(act);
                                        }
                                    }

                                    if jump_to_bottom_clicked {
                                        surface.scroll_to_bottom();
                                    }
                                }
                            }
                            any_animating |=
                                self.rect_animator.render_closing_ghosts(ui.painter(), now_instant);
                        }
                    }
                    }

                    // Hide WebView2 native child HWNDs for inactive tabs/workspaces or obstructed panes.
                    for (&wv_pid, wv) in &mut self.webview_panes {
                        if !visible_webviews.contains(&wv_pid) {
                            wv.sync_layout_and_visibility(false, (0, 0, 0, 0));
                        }
                    }

                    if let Some(url) = url_to_open {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }

                    if let Some(b_act) = pending_browser_action {
                        match b_act {
                            BrowserPaneAction::SwitchToTerminal(pid) => {
                                self.webview_panes.remove(&pid);
                                self.focus_pane_in_active_tab(pid);
                            }
                            BrowserPaneAction::OpenExternal(url) => {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                            }
                        }
                    }

                    if let Some(a_act) = pending_acp_action {
                        match a_act {
                            AcpPaneAction::Restart(pid) => {
                                self.open_acp_in_pane(pid);
                            }
                            AcpPaneAction::SwitchToTerminal(pid) => {
                                self.acp_panes.remove(&pid);
                                self.pane_status.insert(pid, AgentStatus::Idle);
                                self.focus_pane_in_active_tab(pid);
                            }
                        }
                    }

                    if let Some(pid) = clicked_focus_pane {
                        self.focus_pane_in_active_tab(pid);
                    }

                    if let Some(exit_action) = pending_exit_action {
                        match exit_action {
                            ExitBannerAction::RestartShell(pid) => {
                                self.restart_pane_shell(pid);
                            }
                            ExitBannerAction::ResumeAgent(pid) => {
                                self.resume_agent_in_pane(pid);
                            }
                            ExitBannerAction::ClosePane(pid) => {
                                self.focus_pane_in_active_tab(pid);
                                self.close_active_pane();
                            }
                        }
                    }

                    if let Some(pane_action) = pending_pane_action {
                        match pane_action {
                            PaneHeaderAction::LaunchAgy(pid) => {
                                self.agent_launcher_modal
                                    .open(pid, self.config.general.enforce_agy_sandbox);
                            }
                            PaneHeaderAction::SplitRight(pid) => {
                                self.focus_pane_in_active_tab(pid);
                                self.split_active_pane(SplitDirection::Horizontal);
                            }
                            PaneHeaderAction::SplitDown(pid) => {
                                self.focus_pane_in_active_tab(pid);
                                self.split_active_pane(SplitDirection::Vertical);
                            }
                            PaneHeaderAction::ToggleZoom(pid) => {
                                self.focus_pane_in_active_tab(pid);
                                self.toggle_zoom_active();
                            }
                            PaneHeaderAction::Close(pid) => {
                                self.focus_pane_in_active_tab(pid);
                                self.close_active_pane();
                            }
                        }
                    }
            });

        // 5. Floating Modals & Context Menu Overlays (render over all panels)
        if let Some(action) = self.command_palette.show(ui.ctx()) {
            self.execute_palette_action(action);
        }

        // 5. New Workspace Modal (Local / WSL / SSH Backend)
        if let Some(ws_req) = self
            .new_workspace_modal
            .show(ui.ctx(), self.config.general.allowed_root_dir.as_deref())
        {
            let _ = self.create_workspace_with_backend(ws_req);
        }

        // 6. Rename Workspace / Tab Modal
        if let Some((target, new_name)) = self.rename_modal.show(ui.ctx()) {
            match target {
                RenameTarget::Workspace(idx) => {
                    if let Some(ws) = self.workspaces.get_mut(idx) {
                        if ws.rename(new_name) {
                            self.save_session();
                        }
                    }
                }
                RenameTarget::Tab {
                    ws_index,
                    tab_index,
                } => {
                    if let Some(ws) = self.workspaces.get_mut(ws_index) {
                        if let Some(tab) = ws.tabs.get_mut(tab_index) {
                            if tab.rename(new_name) {
                                self.save_session();
                            }
                        }
                    }
                }
            }
        }

        // 7. Comprehensive Visual Settings Modal (Appearance / Terminal & Alerts / Shortcuts / Quick Actions)
        if self
            .settings_modal
            .show(ui.ctx(), &mut self.config, &mut self.preflight)
        {
            self.font_size = self.config.general.font_size.clamp(9.0, 32.0);
            self.command_palette.sync_builtins(&self.config.keybindings);
            self.command_palette
                .set_custom_commands(&self.config.commands);
            let _ = self.config.save_default();
        }

        // 7b. Plain-English Visual Agent Preset Launcher (REQ-076, REQ-080, REQ-082)
        let active_cwd = self
            .active_workspace()
            .map(|w| w.cwd.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        if let Some(req) = self.agent_launcher_modal.show(
            ui.ctx(),
            &mut self.preflight,
            &active_cwd,
            self.config.general.allowed_root_dir.as_deref(),
        ) {
            self.launch_agent_request(req);
        }

        // 7c. Multi-Agent Team Grid Launcher (Unit 7 §C)
        if let Some(team_req) = self.team_launcher_modal.show(ui.ctx()) {
            self.launch_team_grid(team_req);
        }

        // 7d. Safe Multi-Session Prompt Broadcast Modal (REQ-088)
        if self.broadcast_modal.is_open {
            let targets = self.gather_broadcast_targets();
            if let Some(b_req) = self.broadcast_modal.show(ui.ctx(), &targets) {
                self.execute_broadcast(b_req);
            }
        }

        // 8. Right-Click Context Menu overlay (Copy / Paste / Clear / Split)
        let has_selection = self
            .text_selection
            .map(|s| s.is_non_empty() && Some(s.pane_id) == self.context_menu.pane_id)
            .unwrap_or(false);
        if let Some(ctx_action) =
            render_context_menu(ui.ctx(), &mut self.context_menu, has_selection)
        {
            match ctx_action {
                ContextMenuAction::Copy(pid) => {
                    self.copy_selection_to_clipboard(ui.ctx(), Some(pid));
                }
                ContextMenuAction::Paste(pid) => {
                    self.focus_pane_in_active_tab(pid);
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
                ContextMenuAction::Clear(pid) => {
                    if let Some(surface) = self.surfaces.get_mut(&pid) {
                        surface.clear_screen();
                    }
                    self.text_selection = None;
                }
                ContextMenuAction::SplitRight(pid) => {
                    self.focus_pane_in_active_tab(pid);
                    self.split_active_pane(SplitDirection::Horizontal);
                }
                ContextMenuAction::SplitDown(pid) => {
                    self.focus_pane_in_active_tab(pid);
                    self.split_active_pane(SplitDirection::Vertical);
                }
                ContextMenuAction::LaunchAgy(pid) => self
                    .agent_launcher_modal
                    .open(pid, self.config.general.enforce_agy_sandbox),
            }
        }
        self.any_pane_animating = any_animating;
    }
}

fn compute_terminal_grid_size(
    pane_width: f32,
    pane_height: f32,
    header_height: f32,
    pad: f32,
    char_width: f32,
    char_height: f32,
) -> TerminalSize {
    let target_w = (pane_width - (pad * 2.0)).max(50.0);
    let target_h = ((pane_height - header_height).max(40.0) - (pad * 2.0)).max(32.0);
    let cols = (target_w / char_width.max(1.0)).floor().clamp(2.0, 500.0) as u16;
    let rows = (target_h / char_height.max(1.0)).floor().clamp(2.0, 200.0) as u16;
    TerminalSize::new(cols, rows)
}

fn paths_match(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

fn is_safe_conversation_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

fn egui_key_to_code(key: Key) -> Option<KeyCode> {
    match key {
        Key::A => Some(KeyCode::Char('A')),
        Key::B => Some(KeyCode::Char('B')),
        Key::C => Some(KeyCode::Char('C')),
        Key::D => Some(KeyCode::Char('D')),
        Key::E => Some(KeyCode::Char('E')),
        Key::F => Some(KeyCode::Char('F')),
        Key::G => Some(KeyCode::Char('G')),
        Key::H => Some(KeyCode::Char('H')),
        Key::I => Some(KeyCode::Char('I')),
        Key::J => Some(KeyCode::Char('J')),
        Key::K => Some(KeyCode::Char('K')),
        Key::L => Some(KeyCode::Char('L')),
        Key::M => Some(KeyCode::Char('M')),
        Key::N => Some(KeyCode::Char('N')),
        Key::O => Some(KeyCode::Char('O')),
        Key::P => Some(KeyCode::Char('P')),
        Key::Q => Some(KeyCode::Char('Q')),
        Key::R => Some(KeyCode::Char('R')),
        Key::S => Some(KeyCode::Char('S')),
        Key::T => Some(KeyCode::Char('T')),
        Key::U => Some(KeyCode::Char('U')),
        Key::V => Some(KeyCode::Char('V')),
        Key::W => Some(KeyCode::Char('W')),
        Key::X => Some(KeyCode::Char('X')),
        Key::Y => Some(KeyCode::Char('Y')),
        Key::Z => Some(KeyCode::Char('Z')),
        Key::Num0 => Some(KeyCode::Char('0')),
        Key::Num1 => Some(KeyCode::Char('1')),
        Key::Num2 => Some(KeyCode::Char('2')),
        Key::Num3 => Some(KeyCode::Char('3')),
        Key::Num4 => Some(KeyCode::Char('4')),
        Key::Num5 => Some(KeyCode::Char('5')),
        Key::Num6 => Some(KeyCode::Char('6')),
        Key::Num7 => Some(KeyCode::Char('7')),
        Key::Num8 => Some(KeyCode::Char('8')),
        Key::Num9 => Some(KeyCode::Char('9')),
        Key::Enter => Some(KeyCode::Enter),
        Key::Escape => Some(KeyCode::Escape),
        Key::Tab => Some(KeyCode::Tab),
        Key::Backspace => Some(KeyCode::Backspace),
        Key::Space => Some(KeyCode::Space),
        Key::ArrowUp => Some(KeyCode::ArrowUp),
        Key::ArrowDown => Some(KeyCode::ArrowDown),
        Key::ArrowLeft => Some(KeyCode::ArrowLeft),
        Key::ArrowRight => Some(KeyCode::ArrowRight),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::compute_terminal_grid_size;

    #[test]
    fn test_compute_terminal_grid_size_scales_with_viewport() {
        let small = compute_terminal_grid_size(900.0, 560.0, 26.0, 4.0, 8.4, 18.0);
        let large = compute_terminal_grid_size(1680.0, 940.0, 26.0, 4.0, 8.4, 18.0);
        assert!(large.cols > small.cols);
        assert!(large.rows > small.rows);
        assert!(small.cols >= 100);
        assert!(small.rows >= 28);
    }
}
