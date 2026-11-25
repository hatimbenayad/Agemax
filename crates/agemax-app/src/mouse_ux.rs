use crate::icons::{icon_small, icon_text, UiIcon};
use agemax_config::theme::ResolvedTheme;
use agemax_core::layout::{PaneId, SplitDirection};
use agemax_core::session::SessionSnapshot;
use agemax_term::vt::GridPoint;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// Active click-and-drag terminal text selection anchored to logical scrollback lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub pane_id: PaneId,
    pub anchor: GridPoint,
    pub end: GridPoint,
    pub is_dragging: bool,
}

impl TextSelection {
    pub fn contains_cell(&self, pane_id: PaneId, abs_line: i32, col: usize) -> bool {
        if self.pane_id != pane_id || self.anchor == self.end {
            return false;
        }
        let (a, b) = if self.anchor <= self.end {
            (self.anchor, self.end)
        } else {
            (self.end, self.anchor)
        };
        if abs_line < a.line || abs_line > b.line {
            return false;
        }
        if a.line == b.line {
            return col >= a.col && col <= b.col;
        }
        if abs_line == a.line {
            col >= a.col
        } else if abs_line == b.line {
            col <= b.col
        } else {
            true
        }
    }

    pub fn is_non_empty(&self) -> bool {
        self.anchor != self.end
    }
}

/// Active split-divider drag state.
#[derive(Debug, Clone)]
pub struct ActiveSplitDrag {
    pub path: Vec<u8>,
    pub direction: SplitDirection,
    pub container: Rect,
}

/// Right-click terminal context menu state & emitted actions.
#[derive(Debug, Default, Clone)]
pub struct ContextMenuState {
    pub is_open: bool,
    pub pane_id: Option<PaneId>,
    pub pos: Pos2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    Copy(PaneId),
    Paste(PaneId),
    Clear(PaneId),
    SplitRight(PaneId),
    SplitDown(PaneId),
    LaunchAgy(PaneId),
}

/// Startup restore banner choices (`REQ-073`, `REQ-050`, `REQ-051`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRestoreAction {
    RestoreWorkspacesOnly,
    RestoreAndResumeAgents,
    Dismiss,
}

/// Inline exited-pane banner choices (`REQ-074`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitBannerAction {
    RestartShell(PaneId),
    ResumeAgent(PaneId),
    ClosePane(PaneId),
}

/// Translate pointer position inside a terminal canvas into `(viewport_row, col)`.
pub fn viewport_cell_at_pos(
    pos: Pos2,
    origin: Pos2,
    char_width: f32,
    char_height: f32,
    rows: usize,
    cols: usize,
) -> Option<(usize, usize)> {
    if rows == 0 || cols == 0 || char_width <= 0.0 || char_height <= 0.0 {
        return None;
    }
    let dx = pos.x - origin.x;
    let dy = pos.y - origin.y;
    if dx < 0.0 || dy < 0.0 {
        return None;
    }
    let col = ((dx / char_width).floor() as usize).min(cols - 1);
    let row = ((dy / char_height).floor() as usize).min(rows - 1);
    Some((row, col))
}

/// Draw the non-blocking startup session restore banner if a previous `session.json` exists.
pub fn render_session_restore_banner(
    ui: &mut egui::Ui,
    snapshot: &SessionSnapshot,
    theme: &ResolvedTheme,
) -> Option<SessionRestoreAction> {
    let mut action = None;
    let total_ws = snapshot.workspaces.len();
    let saved_agents: usize = snapshot
        .workspaces
        .iter()
        .flat_map(|w| w.tabs.iter())
        .flat_map(|t| t.panes.iter())
        .filter(|p| p.agent_conversation_id.is_some())
        .count();

    let (fill, text_color, border) = if theme.is_light_background() {
        (
            Color32::from_rgb(232, 240, 254),
            Color32::from_rgb(25, 35, 55),
            Color32::from_rgb(80, 125, 220),
        )
    } else {
        (
            Color32::from_rgb(26, 33, 52),
            Color32::from_rgb(225, 232, 245),
            Color32::from_rgb(70, 110, 200),
        )
    };

    let frame = egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, border))
        .inner_margin(Vec2::new(10.0, 6.0));

    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let summary = if saved_agents > 0 {
                format!(
                    "Previous session found ({} workspace{}, {} saved agent chat{}).",
                    total_ws,
                    if total_ws == 1 { "" } else { "s" },
                    saved_agents,
                    if saved_agents == 1 { "" } else { "s" },
                )
            } else {
                format!(
                    "Previous session found ({} workspace{}).",
                    total_ws,
                    if total_ws == 1 { "" } else { "s" },
                )
            };
            ui.label(
                egui::RichText::new(summary)
                    .size(12.0)
                    .color(text_color)
                    .strong(),
            );

            if ui.button("Restore Workspaces").clicked() {
                action = Some(SessionRestoreAction::RestoreWorkspacesOnly);
            }
            if saved_agents > 0 && ui.button("Restore & Resume Agents").clicked() {
                action = Some(SessionRestoreAction::RestoreAndResumeAgents);
            }
            if ui.small_button("Dismiss").clicked() {
                action = Some(SessionRestoreAction::Dismiss);
            }
        });
    });

    action
}

/// Render an inline recovery bar at the bottom of an exited terminal pane (`REQ-074`).
pub fn render_pane_exit_banner(
    ui: &mut egui::Ui,
    pane_rect: Rect,
    pane_id: PaneId,
    has_conversation: bool,
) -> Option<ExitBannerAction> {
    let mut action = None;
    let bar_h = 30.0;
    if pane_rect.height() < bar_h + 40.0 {
        return None;
    }

    let bar_rect = Rect::from_min_max(
        Pos2::new(pane_rect.min.x + 2.0, pane_rect.max.y - bar_h - 2.0),
        Pos2::new(pane_rect.max.x - 2.0, pane_rect.max.y - 2.0),
    );
    ui.painter().rect_filled(
        bar_rect,
        4.0,
        Color32::from_rgba_unmultiplied(32, 24, 28, 235),
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("pane_exit_bar", pane_id.0))
            .max_rect(bar_rect.shrink2(Vec2::new(8.0, 3.0))),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Session ended")
                        .size(11.5)
                        .color(Color32::from_rgb(245, 190, 110))
                        .strong(),
                );
                if ui
                    .small_button(icon_small(UiIcon::RotateCw, "Restart Shell"))
                    .clicked()
                {
                    action = Some(ExitBannerAction::RestartShell(pane_id));
                }
                if has_conversation
                    && ui
                        .small_button(icon_small(UiIcon::Play, "Resume Agent"))
                        .clicked()
                {
                    action = Some(ExitBannerAction::ResumeAgent(pane_id));
                }
                if ui
                    .small_button(icon_small(UiIcon::X, "Close Pane"))
                    .clicked()
                {
                    action = Some(ExitBannerAction::ClosePane(pane_id));
                }
            });
        },
    );

    action
}

/// Render the right-click terminal context menu when active.
pub fn render_context_menu(
    ctx: &egui::Context,
    menu: &mut ContextMenuState,
    has_selection: bool,
) -> Option<ContextMenuAction> {
    if !menu.is_open {
        return None;
    }
    let pane_id = menu.pane_id?;
    let mut emitted = None;
    let mut close = false;

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        menu.is_open = false;
        return None;
    }

    let area_resp = egui::Area::new(egui::Id::unique("terminal_context_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(menu.pos)
        .show(ctx, |ui| {
            let frame = egui::Frame::new()
                .fill(Color32::from_rgb(26, 30, 42))
                .corner_radius(6.0)
                .stroke(Stroke::new(1.0, Color32::from_rgb(70, 110, 200)))
                .inner_margin(6.0);
            frame.show(ui, |ui| {
                ui.set_min_width(155.0);
                ui.vertical(|ui| {
                    if ui
                        .add_enabled(has_selection, egui::Button::new("Copy  (Ctrl+Shift+C)"))
                        .clicked()
                    {
                        emitted = Some(ContextMenuAction::Copy(pane_id));
                        close = true;
                    }
                    if ui.button("Paste  (Ctrl+Shift+V)").clicked() {
                        emitted = Some(ContextMenuAction::Paste(pane_id));
                        close = true;
                    }
                    if ui.button("Clear Screen").clicked() {
                        emitted = Some(ContextMenuAction::Clear(pane_id));
                        close = true;
                    }
                    ui.separator();
                    if ui.button("Split Right").clicked() {
                        emitted = Some(ContextMenuAction::SplitRight(pane_id));
                        close = true;
                    }
                    if ui.button("Split Down").clicked() {
                        emitted = Some(ContextMenuAction::SplitDown(pane_id));
                        close = true;
                    }
                    if ui
                        .button(icon_text(UiIcon::Play, "Start Agent (agy)"))
                        .clicked()
                    {
                        emitted = Some(ContextMenuAction::LaunchAgy(pane_id));
                        close = true;
                    }
                });
            });
        });

    if close
        || (ctx.input(|i| i.pointer.any_click())
            && !area_resp.response.rect.contains(
                ctx.input(|i| i.pointer.interact_pos())
                    .unwrap_or(Pos2::ZERO),
            ))
    {
        menu.is_open = false;
    }

    emitted
}

pub fn color32_from_rgb(rgb: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(rgb.0, rgb.1, rgb.2)
}
