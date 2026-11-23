mod command_palette;
mod ipc_bridge;
mod notification_win;
mod sidebar;
mod tab_bar;

use agemax_agy::osc::{AgentOscEvent, OscParser};
use agemax_agy::profile::ProfileManager;
use agemax_config::hot_reload::ConfigWatcher;
use agemax_config::schema::AgeMaxConfig;
use agemax_core::layout::{Direction, PaneId, SplitDirection, WorkspaceId};
use agemax_core::model::{Point, Rect as CoreRect, TerminalSize};
use agemax_core::notification::NotificationStore;
use agemax_core::session::{
    LayoutSnapshot, PaneSnapshot, SessionSnapshot, TabSnapshot, WorkspaceSnapshot,
};
use agemax_core::status::AgentStatus;
use agemax_core::workspace::{Tab, Workspace};
use agemax_term::metadata::detect_git_branch;
use agemax_term::pty::{detect_default_shell, PtyConfig};
use agemax_term::surface::TerminalSurface;
use command_palette::{CommandPalette, PaletteAction};
use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Stroke, StrokeKind, Vec2};
use ipc_bridge::{start_ipc_server, IpcAppCommand};
use notification_win::{NotificationService, PlatformNotificationService};
use sidebar::{render_sidebar, SidebarAction, SidebarProps, SidebarWorkspaceMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tab_bar::{render_tab_bar, TabBarAction};

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Age Max - Native Agent Multiplexer")
            .with_inner_size([1150.0, 720.0])
            .with_min_inner_size([450.0, 320.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AgeMax",
        native_options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(AgeMaxApp::new(Some(cc))))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

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
    osc_parsers: HashMap<PaneId, OscParser>,
    pane_status: HashMap<PaneId, AgentStatus>,
    agent_conversation_ids: HashMap<PaneId, String>,
    notification_store: NotificationStore,
    notification_service: Box<dyn NotificationService>,
    ipc_rx: crossbeam_channel::Receiver<IpcAppCommand>,
    config: AgeMaxConfig,
    config_watcher: Option<ConfigWatcher>,
    command_palette: CommandPalette,
    show_notifications_panel: bool,
    is_sidebar_collapsed: bool,
    font_size: f32,
    char_width: f32,
    char_height: f32,
    start_time: Instant,
    first_frame_rendered: bool,
}

impl Default for AgeMaxApp {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AgeMaxApp {
    pub fn new(cc: Option<&eframe::CreationContext>) -> Self {
        let start_time = Instant::now();
        let initial_pane = PaneId::new();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let git_branch = detect_git_branch(&cwd);

        let mut initial_workspace = Workspace::new("Main Workspace", cwd.clone(), initial_pane);
        initial_workspace.git_branch = git_branch;

        let initial_surface = spawn_terminal_surface(TerminalSize::new(80, 24), &cwd)
            .expect("Failed to initialize initial ConPTY surface");

        let mut surfaces = HashMap::new();
        surfaces.insert(initial_pane, initial_surface);

        let mut osc_parsers = HashMap::new();
        osc_parsers.insert(initial_pane, OscParser::new());

        let mut pane_status = HashMap::new();
        pane_status.insert(initial_pane, AgentStatus::Idle);

        let (ipc_tx, ipc_rx) = crossbeam_channel::unbounded();
        if let Some(ctx_container) = cc {
            start_ipc_server(ipc_tx, ctx_container.egui_ctx.clone());
        }

        let mut config = AgeMaxConfig::load_or_create_default();
        config.merge_workspace_config(&cwd);

        let config_watcher = ConfigWatcher::watch(AgeMaxConfig::default_path()).ok();
        let mut command_palette = CommandPalette::new();
        command_palette.set_custom_commands(&config.commands);

        Self {
            workspaces: vec![initial_workspace],
            active_workspace_index: 0,
            surfaces,
            osc_parsers,
            pane_status,
            agent_conversation_ids: HashMap::new(),
            notification_store: NotificationStore::new(),
            notification_service: Box::new(PlatformNotificationService::new()),
            ipc_rx,
            config,
            config_watcher,
            command_palette,
            show_notifications_panel: false,
            is_sidebar_collapsed: false,
            font_size: 14.0,
            char_width: 8.4,
            char_height: 18.0,
            start_time,
            first_frame_rendered: false,
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

    pub fn save_session(&self) {
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
                    profile: None,
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
            PaletteAction::SplitRight => self.split_active_pane(SplitDirection::Horizontal),
            PaletteAction::SplitDown => self.split_active_pane(SplitDirection::Vertical),
            PaletteAction::ClosePane => self.close_active_pane(),
            PaletteAction::ToggleZoom => {
                if let Some(ws) = self.active_workspace_mut() {
                    if let Some(tab) = ws.active_tab_mut() {
                        tab.toggle_zoom();
                    }
                }
            }
            PaletteAction::ToggleSidebar => {
                self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
            }
            PaletteAction::JumpUnread => {
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
            PaletteAction::ToggleNotifications => {
                self.show_notifications_panel = !self.show_notifications_panel;
            }
            PaletteAction::NewWorkspace => {
                let pane_id = PaneId::new();
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let git_branch = detect_git_branch(&cwd);
                let mut ws = Workspace::new(
                    format!("Workspace {}", self.workspaces.len() + 1),
                    cwd.clone(),
                    pane_id,
                );
                ws.git_branch = git_branch;
                if let Ok(surface) = spawn_terminal_surface(TerminalSize::new(80, 24), &cwd) {
                    self.surfaces.insert(pane_id, surface);
                    self.osc_parsers.insert(pane_id, OscParser::new());
                    self.pane_status.insert(pane_id, AgentStatus::Idle);
                    self.workspaces.push(ws);
                    self.active_workspace_index = self.workspaces.len() - 1;
                    self.save_session();
                }
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
        let (active_pane, cwd) = match self.active_workspace() {
            Some(ws) => match ws.active_pane_id() {
                Some(p) => (p, ws.cwd.clone()),
                None => return,
            },
            None => return,
        };

        let new_pane_id = PaneId::new();
        if let Ok(surface) = spawn_terminal_surface(TerminalSize::new(80, 24), &cwd) {
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
            self.surfaces.remove(&pane);
            self.osc_parsers.remove(&pane);
            self.pane_status.remove(&pane);
            self.agent_conversation_ids.remove(&pane);
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
                let pane_id = PaneId::new();
                let dir = cwd.map(PathBuf::from).unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
                let ws_name =
                    name.unwrap_or_else(|| format!("Workspace {}", self.workspaces.len() + 1));
                let mut ws = Workspace::new(ws_name, dir.clone(), pane_id);
                ws.git_branch = detect_git_branch(&dir);

                if let Ok(surface) = spawn_terminal_surface(TerminalSize::new(80, 24), &dir) {
                    self.surfaces.insert(pane_id, surface);
                    self.osc_parsers.insert(pane_id, OscParser::new());
                    self.pane_status.insert(pane_id, AgentStatus::Idle);
                    self.workspaces.push(ws);
                    self.active_workspace_index = self.workspaces.len() - 1;
                    self.save_session();

                    let last_ws = self.workspaces.last().unwrap();
                    let _ = reply.send(serde_json::json!({
                        "id": last_ws.id.0.to_string(),
                        "name": last_ws.name
                    }));
                } else {
                    let _ = reply.send(serde_json::json!({ "error": "Failed to spawn surface" }));
                }
            }
            IpcAppCommand::WorkspaceClose { id, reply } => {
                if let Some(idx) = self.workspaces.iter().position(|w| w.id == id) {
                    if self.workspaces.len() > 1 {
                        let ws = self.workspaces.remove(idx);
                        for tab in ws.tabs {
                            for p in tab.layout.panes() {
                                self.surfaces.remove(&p);
                                self.osc_parsers.remove(&p);
                                self.pane_status.remove(&p);
                                self.agent_conversation_ids.remove(&p);
                            }
                        }
                        if self.active_workspace_index >= self.workspaces.len() {
                            self.active_workspace_index = self.workspaces.len().saturating_sub(1);
                        }
                        self.save_session();
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
                if let Some(item) = self
                    .notification_store
                    .latest_unread_for_workspace(active_ws_id)
                {
                    self.notification_service.show_notification(item);
                }
                self.notification_service.flash_window();
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
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context, content_rect: CoreRect) {
        let events = ctx.input(|i| i.events.clone());
        let focused_pane_id = self.active_pane_id();

        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if let Some(id) = focused_pane_id {
                        self.notification_store.mark_pane_read(id);
                        if self.pane_status.get(&id) == Some(&AgentStatus::WaitingForInput) {
                            self.pane_status.insert(id, AgentStatus::Working);
                        }
                        if let Some(surface) = self.surfaces.get_mut(&id) {
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
                    // Global Shortcuts:
                    if modifiers.ctrl && !modifiers.shift && key == Key::B {
                        self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
                        continue;
                    }

                    if modifiers.ctrl && modifiers.shift {
                        match key {
                            Key::P => {
                                self.command_palette.open();
                                continue;
                            }
                            Key::D => {
                                self.split_active_pane(SplitDirection::Horizontal);
                                continue;
                            }
                            Key::R => {
                                self.split_active_pane(SplitDirection::Vertical);
                                continue;
                            }
                            Key::W => {
                                self.close_active_pane();
                                continue;
                            }
                            Key::Enter => {
                                if let Some(ws) = self.active_workspace_mut() {
                                    if let Some(tab) = ws.active_tab_mut() {
                                        tab.toggle_zoom();
                                    }
                                }
                                continue;
                            }
                            Key::U => {
                                // Jump to oldest unread waiting pane
                                if let Some((ws_id, pane_id)) =
                                    self.notification_store.oldest_unread_pane()
                                {
                                    if let Some(idx) =
                                        self.workspaces.iter().position(|w| w.id == ws_id)
                                    {
                                        self.active_workspace_index = idx;
                                        if let Some(ws) = self.workspaces.get_mut(idx) {
                                            if let Some(tab) = ws.active_tab_mut() {
                                                tab.active_pane = pane_id;
                                            }
                                        }
                                        self.notification_store.mark_pane_read(pane_id);
                                    }
                                }
                                continue;
                            }
                            Key::N => {
                                self.show_notifications_panel = !self.show_notifications_panel;
                                continue;
                            }
                            Key::V => {
                                // Paste
                                if let Some(clipboard) = ctx.input(|i| {
                                    i.events.iter().find_map(|e| {
                                        if let egui::Event::Paste(text) = e {
                                            Some(text.clone())
                                        } else {
                                            None
                                        }
                                    })
                                }) {
                                    if let Some(id) = focused_pane_id {
                                        self.notification_store.mark_pane_read(id);
                                        if self.pane_status.get(&id)
                                            == Some(&AgentStatus::WaitingForInput)
                                        {
                                            self.pane_status.insert(id, AgentStatus::Working);
                                        }
                                        if let Some(surface) = self.surfaces.get_mut(&id) {
                                            let _ = surface.send_input(clipboard.as_bytes());
                                        }
                                    }
                                }
                                continue;
                            }
                            _ => {}
                        }
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
                    if let Some(id) = focused_pane_id {
                        self.notification_store.mark_pane_read(id);
                        if self.pane_status.get(&id) == Some(&AgentStatus::WaitingForInput) {
                            self.pane_status.insert(id, AgentStatus::Working);
                        }

                        if let Some(surface) = self.surfaces.get_mut(&id) {
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
                    if let Some(id) = focused_pane_id {
                        self.notification_store.mark_pane_read(id);
                        if self.pane_status.get(&id) == Some(&AgentStatus::WaitingForInput) {
                            self.pane_status.insert(id, AgentStatus::Working);
                        }
                        if let Some(surface) = self.surfaces.get_mut(&id) {
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
) -> Result<TerminalSurface, agemax_term::PtyError> {
    let shell = detect_default_shell();
    let config = PtyConfig {
        program: shell,
        args: Vec::new(),
        env: HashMap::new(),
        cwd: Some(cwd.to_path_buf()),
        cols: size.cols,
        rows: size.rows,
    };
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

        // 1. Check for configuration hot reload
        if let Some(watcher) = &self.config_watcher {
            if let Some(new_cfg) = watcher.try_recv() {
                self.command_palette.set_custom_commands(&new_cfg.commands);
                self.config = new_cfg;
            }
        }

        // 2. Process pending IPC commands
        while let Ok(cmd) = self.ipc_rx.try_recv() {
            self.handle_ipc_command(cmd);
        }

        // 3. Process output for all active surfaces and parse OSC sequences
        let mut dirty = false;
        let mut events_to_process = Vec::new();

        for (&pane_id, surface) in self.surfaces.iter_mut() {
            let parser = self.osc_parsers.entry(pane_id).or_default();
            if surface.process_output_with(|chunk| {
                let evs = parser.parse(chunk);
                if !evs.is_empty() {
                    events_to_process.push((pane_id, evs));
                }
            }) {
                dirty = true;
            }
        }

        for (pane_id, events) in events_to_process {
            for event in events {
                match event {
                    AgentOscEvent::StatusChange { status, message } => {
                        self.pane_status.insert(pane_id, status);
                        if status.is_attention_needed() {
                            if let Some(ws_id) = self.find_workspace_for_pane(pane_id) {
                                let msg =
                                    message.unwrap_or_else(|| status.display_name().to_string());
                                self.notification_store.notify(ws_id, pane_id, status, &msg);
                                if let Some(item) =
                                    self.notification_store.latest_unread_for_workspace(ws_id)
                                {
                                    self.notification_service.show_notification(item);
                                }
                                self.notification_service.flash_window();
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
                            if let Some(item) =
                                self.notification_store.latest_unread_for_workspace(ws_id)
                            {
                                self.notification_service.show_notification(item);
                            }
                            self.notification_service.flash_window();
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
                            if let Some(item) =
                                self.notification_store.latest_unread_for_workspace(ws_id)
                            {
                                self.notification_service.show_notification(item);
                            }
                            self.notification_service.flash_window();
                        }
                    }
                }
            }
        }

        if dirty {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let full_frame = egui::Frame::new().fill(Color32::from_rgb(15, 17, 23));
        full_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Prepare sidebar metadata
                let sidebar_meta: Vec<SidebarWorkspaceMeta> = self
                    .workspaces
                    .iter()
                    .map(|ws| {
                        let unread_count =
                            self.notification_store.unread_count_for_workspace(ws.id);
                        let latest_snippet = self
                            .notification_store
                            .latest_unread_for_workspace(ws.id)
                            .map(|i| i.message.clone());
                        SidebarWorkspaceMeta {
                            unread_count,
                            latest_snippet,
                            status: None,
                        }
                    })
                    .collect();

                // 1. Sidebar on the left
                let sidebar_action = render_sidebar(
                    ui,
                    SidebarProps {
                        workspaces: &self.workspaces,
                        active_index: self.active_workspace_index,
                        is_collapsed: self.is_sidebar_collapsed,
                        meta: &sidebar_meta,
                    },
                );

                if let Some(action) = sidebar_action {
                    match action {
                        SidebarAction::SelectWorkspace(idx) => {
                            self.active_workspace_index = idx;
                        }
                        SidebarAction::CreateWorkspace => {
                            let pane_id = PaneId::new();
                            let cwd =
                                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                            let git_branch = detect_git_branch(&cwd);
                            let mut ws = Workspace::new(
                                format!("Workspace {}", self.workspaces.len() + 1),
                                cwd.clone(),
                                pane_id,
                            );
                            ws.git_branch = git_branch;
                            if let Ok(surface) =
                                spawn_terminal_surface(TerminalSize::new(80, 24), &cwd)
                            {
                                self.surfaces.insert(pane_id, surface);
                                self.osc_parsers.insert(pane_id, OscParser::new());
                                self.pane_status.insert(pane_id, AgentStatus::Idle);
                                self.workspaces.push(ws);
                                self.active_workspace_index = self.workspaces.len() - 1;
                                self.save_session();
                            }
                        }
                        SidebarAction::ToggleCollapse => {
                            self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
                        }
                    }
                }

                // 2. Main content area (Tab Bar + Terminal Panes)
                ui.vertical(|ui| {
                    let active_ws_idx = self.active_workspace_index;
                    let tab_action = if let Some(ws) = self.workspaces.get(active_ws_idx) {
                        render_tab_bar(ui, &ws.tabs, ws.active_tab_index)
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
                            TabBarAction::Create => {
                                if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                                    let new_pane = PaneId::new();
                                    if let Ok(surface) =
                                        spawn_terminal_surface(TerminalSize::new(80, 24), &ws.cwd)
                                    {
                                        self.surfaces.insert(new_pane, surface);
                                        self.osc_parsers.insert(new_pane, OscParser::new());
                                        self.pane_status.insert(new_pane, AgentStatus::Idle);
                                        let tab_num = ws.tabs.len() + 1;
                                        ws.tabs.push(Tab::new(
                                            format!("Terminal {}", tab_num),
                                            new_pane,
                                        ));
                                        ws.active_tab_index = ws.tabs.len() - 1;
                                        self.save_session();
                                    }
                                }
                            }
                            TabBarAction::Close(idx) => {
                                if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                                    if ws.tabs.len() > 1 {
                                        let closed_tab = ws.tabs.remove(idx);
                                        for p in closed_tab.layout.panes() {
                                            self.surfaces.remove(&p);
                                            self.osc_parsers.remove(&p);
                                            self.pane_status.remove(&p);
                                            self.agent_conversation_ids.remove(&p);
                                        }
                                        if ws.active_tab_index >= ws.tabs.len() {
                                            ws.active_tab_index = ws.tabs.len().saturating_sub(1);
                                        }
                                        self.save_session();
                                    }
                                }
                            }
                        }
                    }

                    // Calculate content area rectangle
                    let content_size = ui.available_size();
                    let content_min = ui.cursor().min;
                    let content_rect = CoreRect::from_min_size(
                        Point::new(content_min.x, content_min.y),
                        content_size.x,
                        content_size.y,
                    );

                    // Handle keyboard input routed to focused pane
                    self.handle_input(ui.ctx(), content_rect);

                    // 3. Render panes inside the tab layout
                    if let Some(ws) = self.workspaces.get(active_ws_idx) {
                        if let Some(tab) = ws.active_tab() {
                            let focused_pane = tab.active_pane;
                            let pane_rects = if let Some(zoomed) = tab.zoomed_pane {
                                let mut m = HashMap::new();
                                m.insert(zoomed, content_rect);
                                m
                            } else {
                                tab.layout.compute_rects(content_rect)
                            };

                            let painter = ui.painter();
                            let font_id = FontId::monospace(self.font_size);
                            let sample = painter.layout_no_wrap(
                                "M".to_string(),
                                font_id.clone(),
                                Color32::WHITE,
                            );
                            self.char_width = sample.size().x.max(4.0);
                            self.char_height = sample.size().y.max(8.0);

                            for (pane_id, rect) in pane_rects {
                                let is_focused = pane_id == focused_pane;
                                let egui_rect = Rect::from_min_size(
                                    Pos2::new(rect.min.x, rect.min.y),
                                    Vec2::new(rect.width(), rect.height()),
                                );

                                // Check click to focus
                                if ui.input(|i| i.pointer.any_click()) {
                                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                                        if egui_rect.contains(pos) {
                                            if let Some(ws_mut) =
                                                self.workspaces.get_mut(active_ws_idx)
                                            {
                                                if let Some(tab_mut) = ws_mut.active_tab_mut() {
                                                    tab_mut.active_pane = pane_id;
                                                }
                                            }
                                        }
                                    }
                                }

                                // Pane border highlight with colored attention ring
                                let pane_status = self
                                    .pane_status
                                    .get(&pane_id)
                                    .copied()
                                    .unwrap_or(AgentStatus::Idle);

                                let (border_color, stroke_width) = match pane_status {
                                    AgentStatus::WaitingForInput => {
                                        (Color32::from_rgb(245, 175, 45), 3.0)
                                    }
                                    AgentStatus::Done => (Color32::from_rgb(45, 200, 110), 2.5),
                                    AgentStatus::Failed => (Color32::from_rgb(235, 60, 60), 3.0),
                                    _ if is_focused => (Color32::from_rgb(70, 110, 200), 1.5),
                                    _ => (Color32::from_rgb(32, 35, 45), 1.0),
                                };

                                painter.rect_stroke(
                                    egui_rect,
                                    2.0,
                                    Stroke::new(stroke_width, border_color),
                                    StrokeKind::Inside,
                                );

                                // Calculate terminal rows and cols for this pane
                                let pad = 4.0;
                                let inner_w = (rect.width() - (pad * 2.0)).max(50.0);
                                let inner_h = (rect.height() - (pad * 2.0)).max(50.0);
                                let cols = (inner_w / self.char_width).floor() as u16;
                                let rows = (inner_h / self.char_height).floor() as u16;

                                if let Some(surface) = self.surfaces.get_mut(&pane_id) {
                                    let new_size = TerminalSize::new(cols, rows);
                                    let _ = surface.resize(new_size);

                                    let origin = Pos2::new(rect.min.x + pad, rect.min.y + pad);
                                    let cur_rows = surface.rows().min(rows as usize);
                                    let cur_cols = surface.cols().min(cols as usize);

                                    for r in 0..cur_rows {
                                        let y = origin.y + (r as f32 * self.char_height);
                                        for c in 0..cur_cols {
                                            let cell = surface.get_cell(r, c);
                                            let x = origin.x + (c as f32 * self.char_width);

                                            if let Some(bg) = cell.bg {
                                                let cell_rect = Rect::from_min_size(
                                                    Pos2::new(x, y),
                                                    Vec2::new(self.char_width, self.char_height),
                                                );
                                                painter.rect_filled(
                                                    cell_rect,
                                                    0.0,
                                                    Color32::from_rgb(bg.r, bg.g, bg.b),
                                                );
                                            }

                                            if cell.c != ' ' && cell.c != '\0' {
                                                let fg_color = if let Some(fg) = cell.fg {
                                                    Color32::from_rgb(fg.r, fg.g, fg.b)
                                                } else {
                                                    Color32::from_rgb(220, 224, 232)
                                                };

                                                painter.text(
                                                    Pos2::new(x, y),
                                                    egui::Align2::LEFT_TOP,
                                                    cell.c.to_string(),
                                                    font_id.clone(),
                                                    fg_color,
                                                );
                                            }
                                        }
                                    }

                                    // Draw cursor if pane is focused
                                    if is_focused {
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
                                            painter.rect_filled(
                                                cursor_rect,
                                                1.0,
                                                Color32::from_rgba_unmultiplied(230, 235, 245, 160),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                });

                // 3. Optional Right-Side Notification History Panel (Ctrl+Shift+N)
                if self.show_notifications_panel {
                    ui.allocate_ui(Vec2::new(260.0, ui.available_height()), |ui| {
                        let frame = egui::Frame::new()
                            .fill(Color32::from_rgb(22, 24, 34))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(40, 44, 60)))
                            .inner_margin(8.0);

                        frame.show(ui, |ui| {
                            ui.heading(
                                egui::RichText::new("NOTIFICATIONS")
                                    .size(12.0)
                                    .color(Color32::from_rgb(160, 165, 180))
                                    .strong(),
                            );
                            ui.separator();
                            ui.add_space(4.0);

                            let unread = self.notification_store.unread_items();
                            if unread.is_empty() {
                                ui.label(
                                    egui::RichText::new("All caught up! No unread notifications.")
                                        .size(11.0)
                                        .color(Color32::GRAY),
                                );
                            } else {
                                let mut jump_target = None;
                                for item in unread {
                                    ui.group(|ui| {
                                        let status_color = match item.status {
                                            AgentStatus::WaitingForInput => {
                                                Color32::from_rgb(245, 175, 45)
                                            }
                                            AgentStatus::Done => Color32::from_rgb(45, 200, 110),
                                            AgentStatus::Failed => Color32::from_rgb(235, 60, 60),
                                            _ => Color32::WHITE,
                                        };

                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} {}",
                                                item.status.icon(),
                                                item.status.display_name()
                                            ))
                                            .size(11.0)
                                            .color(status_color)
                                            .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&item.message)
                                                .size(11.0)
                                                .color(Color32::from_rgb(200, 205, 220)),
                                        );

                                        if ui.button("Focus Pane").clicked() {
                                            jump_target = Some((item.workspace_id, item.pane_id));
                                        }
                                    });
                                    ui.add_space(4.0);
                                }

                                if let Some((ws_id, pane_id)) = jump_target {
                                    if let Some(pos) =
                                        self.workspaces.iter().position(|w| w.id == ws_id)
                                    {
                                        self.active_workspace_index = pos;
                                        if let Some(ws) = self.workspaces.get_mut(pos) {
                                            if let Some(tab) = ws.active_tab_mut() {
                                                tab.active_pane = pane_id;
                                            }
                                        }
                                        self.notification_store.mark_pane_read(pane_id);
                                    }
                                }
                            }
                        });
                    });
                }

                // 4. Command Palette (Ctrl+Shift+P)
                if let Some(action) = self.command_palette.show(ui.ctx()) {
                    self.execute_palette_action(action);
                }
            });
        });
    }
}
