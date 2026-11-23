mod sidebar;
mod tab_bar;

use agemax_core::layout::{Direction, PaneId, SplitDirection};
use agemax_core::model::{Point, Rect as CoreRect, TerminalSize};
use agemax_core::workspace::{Tab, Workspace};
use agemax_term::metadata::detect_git_branch;
use agemax_term::pty::{detect_default_shell, PtyConfig};
use agemax_term::surface::TerminalSurface;
use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Stroke, StrokeKind, Vec2};
use sidebar::{render_sidebar, SidebarAction, SidebarProps};
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
            Ok(Box::new(AgeMaxApp::new()))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "Cascadia Code".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(1, "Consolas".to_owned());
    ctx.set_fonts(fonts);
}

pub struct AgeMaxApp {
    workspaces: Vec<Workspace>,
    active_workspace_index: usize,
    surfaces: HashMap<PaneId, TerminalSurface>,
    is_sidebar_collapsed: bool,
    font_size: f32,
    char_width: f32,
    char_height: f32,
    start_time: Instant,
    first_frame_rendered: bool,
}

impl Default for AgeMaxApp {
    fn default() -> Self {
        Self::new()
    }
}

impl AgeMaxApp {
    pub fn new() -> Self {
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

        Self {
            workspaces: vec![initial_workspace],
            active_workspace_index: 0,
            surfaces,
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

            if let Some(ws) = self.active_workspace_mut() {
                if let Some(tab) = ws.active_tab_mut() {
                    tab.layout.split(active_pane, direction, new_pane_id);
                    tab.active_pane = new_pane_id;
                }
            }
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

    fn handle_input(&mut self, ctx: &egui::Context, content_rect: CoreRect) {
        let events = ctx.input(|i| i.events.clone());
        let focused_pane_id = self.active_pane_id();

        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if let Some(id) = focused_pane_id {
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

        // Process output for all active surfaces
        let mut dirty = false;
        for surface in self.surfaces.values_mut() {
            if surface.process_output() {
                dirty = true;
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
                // 1. Sidebar on the left
                let sidebar_action = render_sidebar(
                    ui,
                    SidebarProps {
                        workspaces: &self.workspaces,
                        active_index: self.active_workspace_index,
                        is_collapsed: self.is_sidebar_collapsed,
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
                                self.workspaces.push(ws);
                                self.active_workspace_index = self.workspaces.len() - 1;
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
                                        let tab_num = ws.tabs.len() + 1;
                                        ws.tabs.push(Tab::new(
                                            format!("Terminal {}", tab_num),
                                            new_pane,
                                        ));
                                        ws.active_tab_index = ws.tabs.len() - 1;
                                    }
                                }
                            }
                            TabBarAction::Close(idx) => {
                                if let Some(ws) = self.workspaces.get_mut(active_ws_idx) {
                                    if ws.tabs.len() > 1 {
                                        let closed_tab = ws.tabs.remove(idx);
                                        for p in closed_tab.layout.panes() {
                                            self.surfaces.remove(&p);
                                        }
                                        if ws.active_tab_index >= ws.tabs.len() {
                                            ws.active_tab_index = ws.tabs.len().saturating_sub(1);
                                        }
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

                                // Pane border highlight
                                let border_color = if is_focused {
                                    Color32::from_rgb(70, 110, 200)
                                } else {
                                    Color32::from_rgb(32, 35, 45)
                                };
                                painter.rect_stroke(
                                    egui_rect,
                                    2.0,
                                    Stroke::new(if is_focused { 1.5 } else { 1.0 }, border_color),
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
            });
        });
    }
}
