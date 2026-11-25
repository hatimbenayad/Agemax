//! Scrollable rounded workspace tab strip with live per-tab status indicators.

use crate::icons::{icon_rich, UiIcon};
use crate::ui_tokens::{metrics, UiTokens};
use agemax_config::theme::ResolvedTheme;
use agemax_core::status::AgentStatus;
use agemax_core::workspace::Tab;
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};

pub enum TabBarAction {
    Select(usize),
    Create,
    Rename(usize),
    Close(usize),
}

pub fn render_tab_bar(
    ui: &mut egui::Ui,
    tabs: &[Tab],
    tab_statuses: &[AgentStatus],
    active_index: usize,
    show_hints: bool,
    theme: &ResolvedTheme,
) -> Option<TabBarAction> {
    let mut action = None;
    let tokens = UiTokens::from_theme(theme);

    let frame = egui::Frame::new()
        .fill(tokens.bg_chrome)
        .inner_margin(Vec2::new(10.0, 5.0));

    let resp = frame.show(ui, |ui| {
        ui.set_min_size(Vec2::new(ui.available_width(), 26.0));
        egui::ScrollArea::horizontal()
            .id_salt("tab_strip_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    for (idx, tab) in tabs.iter().enumerate() {
                        let is_active = idx == active_index;
                        let tab_status =
                            tab_statuses.get(idx).copied().unwrap_or(AgentStatus::Idle);
                        let dot_col = tab_status_dot_color(tab_status, &tokens);

                        let bg_color = if is_active {
                            tokens.bg_elevated
                        } else {
                            tokens.bg_surface
                        };
                        let border_color = if is_active {
                            tokens.border_focus
                        } else {
                            tokens.border_subtle
                        };

                        let tab_frame = egui::Frame::new()
                            .fill(bg_color)
                            .stroke(Stroke::new(1.0, border_color))
                            .corner_radius(metrics::RADIUS_SM)
                            .inner_margin(Vec2::new(9.0, 4.0));

                        tab_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                ui.label(icon_rich(
                                    UiIcon::Terminal,
                                    11.5,
                                    Some(if is_active {
                                        tokens.text_primary
                                    } else {
                                        tokens.text_muted
                                    }),
                                ));

                                let text_color = if is_active {
                                    tokens.text_primary
                                } else {
                                    tokens.text_muted
                                };
                                let title_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&tab.title)
                                            .color(text_color)
                                            .size(12.0),
                                    )
                                    .sense(Sense::click()),
                                );
                                if title_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                let title_resp = if show_hints {
                                    title_resp
                                        .on_hover_text("Click to focus • Double-click to rename")
                                } else {
                                    title_resp
                                };
                                if title_resp.double_clicked() {
                                    action = Some(TabBarAction::Rename(idx));
                                } else if title_resp.clicked() {
                                    action = Some(TabBarAction::Select(idx));
                                }

                                let (dot_rect, _) =
                                    ui.allocate_exact_size(Vec2::new(7.0, 12.0), Sense::hover());
                                ui.painter().circle_filled(dot_rect.center(), 2.8, dot_col);

                                if is_active
                                    && ui
                                        .add(
                                            egui::Button::new(icon_rich(
                                                UiIcon::Pencil,
                                                10.0,
                                                Some(tokens.text_muted),
                                            ))
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE),
                                        )
                                        .on_hover_text("Rename tab")
                                        .clicked()
                                {
                                    action = Some(TabBarAction::Rename(idx));
                                }

                                if tabs.len() > 1
                                    && ui
                                        .add(
                                            egui::Button::new(icon_rich(
                                                UiIcon::X,
                                                10.5,
                                                Some(tokens.text_muted),
                                            ))
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE),
                                        )
                                        .on_hover_text("Close tab")
                                        .clicked()
                                {
                                    action = Some(TabBarAction::Close(idx));
                                }
                            });
                        });
                    }

                    let add_btn = ui.add(
                        egui::Button::new(icon_rich(UiIcon::Plus, 12.0, Some(tokens.text_muted)))
                            .fill(tokens.bg_surface)
                            .stroke(Stroke::new(1.0, tokens.border_subtle))
                            .corner_radius(metrics::RADIUS_SM)
                            .min_size(Vec2::new(26.0, 24.0)),
                    );
                    let add_btn = if show_hints {
                        add_btn.on_hover_text("New Tab")
                    } else {
                        add_btn
                    };
                    if add_btn.clicked() {
                        action = Some(TabBarAction::Create);
                    }
                });
            });
    });

    let outer = resp.response.rect;
    ui.painter().line_segment(
        [
            Pos2::new(outer.min.x, outer.max.y),
            Pos2::new(outer.max.x, outer.max.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );

    action
}

fn tab_status_dot_color(status: AgentStatus, tokens: &UiTokens) -> Color32 {
    match status {
        AgentStatus::Working
        | AgentStatus::Thinking
        | AgentStatus::RunningTool
        | AgentStatus::Done => tokens.status_running_fg,
        AgentStatus::WaitingForInput => Color32::from_rgb(245, 158, 11),
        AgentStatus::Failed => Color32::from_rgb(239, 68, 68),
        AgentStatus::Idle => Color32::from_rgb(100, 116, 139),
    }
}
