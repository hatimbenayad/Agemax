use crate::icons::{icon_label, icon_rich, icon_text, UiIcon};
use crate::telemetry::{format_elapsed_compact, render_sparkline, SPARKLINE_BUCKETS};
use crate::ui_tokens::{metrics, UiTokens};
use agemax_config::safety::display_clean_path;
use agemax_config::theme::ResolvedTheme;
use agemax_core::status::AgentStatus;
use agemax_core::workspace::Workspace;
use agemax_term::metadata::GitDiffStat;
use eframe::egui::{self, Color32, Sense, Stroke, Vec2};

#[derive(Debug, Clone, Default)]
pub struct SidebarWorkspaceMeta {
    pub unread_count: usize,
    pub latest_snippet: Option<String>,
    pub last_output_line: Option<String>,
    pub working_elapsed_secs: Option<u64>,
    pub git_stats: Option<GitDiffStat>,
    pub sparkline: [u32; SPARKLINE_BUCKETS],
    pub status: Option<AgentStatus>,
}

pub struct SidebarProps<'a> {
    pub workspaces: &'a [Workspace],
    pub active_index: usize,
    pub is_collapsed: bool,
    pub meta: &'a [SidebarWorkspaceMeta],
    pub theme: &'a ResolvedTheme,
}

pub enum SidebarAction {
    SelectWorkspace(usize),
    CreateWorkspace,
    RenameWorkspace(usize),
    CloseWorkspace(usize),
    ToggleCollapse,
    OpenViewSettings,
    OpenShortcuts,
}

pub fn render_sidebar(ui: &mut egui::Ui, props: SidebarProps) -> Option<SidebarAction> {
    let mut action = None;
    let tokens = UiTokens::from_theme(props.theme);

    let frame = egui::Frame::new()
        .fill(tokens.bg_chrome)
        .stroke(Stroke::new(1.0, tokens.border_subtle))
        .inner_margin(Vec2::new(8.0, 8.0));

    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if !props.is_collapsed {
                    ui.label(
                        egui::RichText::new("WORKSPACES")
                            .size(11.0)
                            .color(tokens.text_muted)
                            .strong(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let toggle_icon = if props.is_collapsed {
                        UiIcon::ChevronsRight
                    } else {
                        UiIcon::ChevronsLeft
                    };
                    if ui
                        .add(
                            egui::Button::new(icon_rich(
                                toggle_icon,
                                13.0,
                                Some(tokens.text_muted),
                            ))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .on_hover_text(if props.is_collapsed {
                            "Expand workspace sidebar"
                        } else {
                            "Collapse workspace sidebar"
                        })
                        .clicked()
                    {
                        action = Some(SidebarAction::ToggleCollapse);
                    }
                });
            });

            ui.add_space(6.0);

            // Scrollable workspace cards fill remaining height above pinned footer
            let footer_reserved_h = if props.is_collapsed { 88.0 } else { 78.0 };
            let list_h = (ui.available_height() - footer_reserved_h).max(60.0);

            egui::ScrollArea::vertical()
                .id_salt("sidebar_workspaces_scroll")
                .auto_shrink([false, false])
                .max_height(list_h)
                .show(ui, |ui| {
                    for (idx, ws) in props.workspaces.iter().enumerate() {
                        let is_active = idx == props.active_index;
                        let meta = props.meta.get(idx).cloned().unwrap_or_default();
                        let has_unread = meta.unread_count > 0;

                        let bg_color = if is_active {
                            tokens.bg_elevated
                        } else if has_unread {
                            Color32::from_rgb(38, 30, 20)
                        } else {
                            tokens.bg_surface
                        };
                        let border_stroke = if is_active {
                            Stroke::new(1.5, tokens.border_focus)
                        } else if has_unread {
                            Stroke::new(1.0, Color32::from_rgb(180, 125, 38))
                        } else {
                            Stroke::new(1.0, tokens.border_subtle)
                        };

                        let card_frame = egui::Frame::new()
                            .fill(bg_color)
                            .corner_radius(metrics::RADIUS_MD)
                            .stroke(border_stroke)
                            .inner_margin(Vec2::new(8.0, 7.0));

                        let dot_color = status_dot_color(meta.status, &tokens);

                        card_frame.show(ui, |ui| {
                            if props.is_collapsed {
                                let first_letter =
                                    ws.name.chars().next().unwrap_or('W').to_string();
                                let text_color = if is_active {
                                    tokens.text_primary
                                } else {
                                    tokens.text_muted
                                };
                                let resp = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(first_letter)
                                            .color(text_color)
                                            .size(13.5)
                                            .strong(),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE)
                                    .min_size(Vec2::new(26.0, 26.0)),
                                );
                                ui.painter().circle_filled(
                                    egui::Pos2::new(resp.rect.max.x - 3.0, resp.rect.min.y + 4.0),
                                    3.0,
                                    dot_color,
                                );
                                if resp.clicked() {
                                    action = Some(SidebarAction::SelectWorkspace(idx));
                                }
                                let tooltip = if has_unread {
                                    format!("{} ({} unread)", ws.name, meta.unread_count)
                                } else {
                                    ws.name.clone()
                                };
                                resp.on_hover_text(tooltip);
                            } else {
                                // Row 1: Colored status dot + Bold name + optional timer/unread + rename/close
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 5.0;
                                    let (dot_rect, dot_resp) = ui
                                        .allocate_exact_size(Vec2::new(8.0, 14.0), Sense::click());
                                    ui.painter()
                                        .circle_filled(dot_rect.center(), 3.8, dot_color);
                                    if dot_resp.clicked() {
                                        action = Some(SidebarAction::SelectWorkspace(idx));
                                    }

                                    let label_color = if has_unread {
                                        Color32::from_rgb(251, 191, 36)
                                    } else {
                                        tokens.text_primary
                                    };
                                    let name_resp = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&ws.name)
                                                .color(label_color)
                                                .size(12.5)
                                                .strong(),
                                        )
                                        .truncate()
                                        .sense(Sense::click()),
                                    );
                                    if name_resp.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if name_resp.clicked() {
                                        action = Some(SidebarAction::SelectWorkspace(idx));
                                    }

                                    if let Some(elapsed) = meta.working_elapsed_secs {
                                        ui.label(
                                            egui::RichText::new(format_elapsed_compact(elapsed))
                                                .size(10.0)
                                                .color(tokens.border_focus)
                                                .strong(),
                                        );
                                    }

                                    if has_unread {
                                        ui.label(
                                            egui::RichText::new(format!("[{}]", meta.unread_count))
                                                .size(10.5)
                                                .color(Color32::from_rgb(251, 191, 36))
                                                .strong(),
                                        );
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            if props.workspaces.len() > 1
                                                && ui
                                                    .small_button(icon_rich(
                                                        UiIcon::X,
                                                        10.0,
                                                        Some(tokens.text_muted),
                                                    ))
                                                    .on_hover_text("Close workspace")
                                                    .clicked()
                                            {
                                                action = Some(SidebarAction::CloseWorkspace(idx));
                                            }
                                            if ui
                                                .small_button(icon_rich(
                                                    UiIcon::Pencil,
                                                    10.0,
                                                    Some(tokens.text_muted),
                                                ))
                                                .on_hover_text("Rename workspace")
                                                .clicked()
                                            {
                                                action = Some(SidebarAction::RenameWorkspace(idx));
                                            }
                                        },
                                    );
                                });

                                // Row 2: Clean folder path or backend badge
                                let path_text = match ws.backend.short_badge() {
                                    Some(badge) => {
                                        format!("[{badge}] {}", display_clean_path(&ws.cwd))
                                    }
                                    None => display_clean_path(&ws.cwd),
                                };
                                let path_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(path_text)
                                            .size(10.0)
                                            .color(tokens.text_muted),
                                    )
                                    .truncate()
                                    .sense(Sense::click()),
                                );
                                if path_resp.clicked() {
                                    action = Some(SidebarAction::SelectWorkspace(idx));
                                }

                                ui.add_space(4.0);

                                // Row 3: Git branch chip + diff churn on left, smooth polyline sparkline on right
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    if let Some(branch) = &ws.git_branch {
                                        egui::Frame::new()
                                            .fill(tokens.bg_chrome)
                                            .stroke(Stroke::new(1.0, tokens.border_subtle))
                                            .corner_radius(4.0)
                                            .inner_margin(Vec2::new(5.0, 1.5))
                                            .show(ui, |ui| {
                                                ui.label(icon_label(
                                                    UiIcon::GitBranch,
                                                    branch,
                                                    10.0,
                                                    Some(tokens.text_primary),
                                                ));
                                            });
                                    }
                                    if let Some(stat) = meta.git_stats {
                                        if stat.has_changes() {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "+{}",
                                                    stat.insertions
                                                ))
                                                .size(10.0)
                                                .color(Color32::from_rgb(74, 222, 128)),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("-{}", stat.deletions))
                                                    .size(10.0)
                                                    .color(Color32::from_rgb(248, 113, 113)),
                                            );
                                        }
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (srect, _) = ui.allocate_exact_size(
                                                Vec2::new(48.0, 13.0),
                                                Sense::hover(),
                                            );
                                            let spark_col = if is_active {
                                                tokens.border_focus
                                            } else if meta
                                                .status
                                                .map(|s| s.is_working())
                                                .unwrap_or(false)
                                            {
                                                tokens.status_running_fg
                                            } else {
                                                tokens.text_muted
                                            };
                                            render_sparkline(
                                                ui.painter(),
                                                srect,
                                                &meta.sparkline,
                                                spark_col,
                                                tokens.border_subtle,
                                            );
                                        },
                                    );
                                });

                                if let Some(snippet) = &meta.latest_snippet {
                                    ui.add_space(2.0);
                                    ui.add(
                                        egui::Label::new(icon_label(
                                            UiIcon::AlertTriangle,
                                            snippet,
                                            10.0,
                                            Some(Color32::from_rgb(251, 191, 36)),
                                        ))
                                        .truncate(),
                                    );
                                } else if let Some(last_line) = &meta.last_output_line {
                                    ui.add_space(2.0);
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("> {last_line}"))
                                                .size(10.0)
                                                .monospace()
                                                .color(tokens.text_muted),
                                        )
                                        .truncate(),
                                    );
                                }
                            }
                        });

                        ui.add_space(4.0);
                    }
                });

            // Pinned bottom actions (+ New Workspace, Settings, Shortcuts)
            ui.separator();
            ui.add_space(6.0);
            if !props.is_collapsed {
                let avail_w = ui.available_width();
                if ui
                    .add(
                        egui::Button::new(icon_text(UiIcon::Plus, "New Workspace"))
                            .fill(tokens.bg_surface)
                            .stroke(Stroke::new(1.0, tokens.border_subtle))
                            .corner_radius(metrics::RADIUS_SM)
                            .min_size(Vec2::new(avail_w, 28.0)),
                    )
                    .clicked()
                {
                    action = Some(SidebarAction::CreateWorkspace);
                }
                ui.add_space(6.0);
                let half_w = ((avail_w - 6.0) * 0.5).max(64.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if ui
                        .add(
                            egui::Button::new(icon_label(
                                UiIcon::Settings,
                                "Settings",
                                11.5,
                                Some(tokens.text_muted),
                            ))
                            .fill(tokens.bg_surface)
                            .stroke(Stroke::new(1.0, tokens.border_subtle))
                            .corner_radius(metrics::RADIUS_SM)
                            .min_size(Vec2::new(half_w, 26.0)),
                        )
                        .on_hover_text("Open Settings (Appearance, Safety, Alerts)")
                        .clicked()
                    {
                        action = Some(SidebarAction::OpenViewSettings);
                    }
                    if ui
                        .add(
                            egui::Button::new(icon_label(
                                UiIcon::Keyboard,
                                "Shortcuts",
                                11.5,
                                Some(tokens.text_muted),
                            ))
                            .fill(tokens.bg_surface)
                            .stroke(Stroke::new(1.0, tokens.border_subtle))
                            .corner_radius(metrics::RADIUS_SM)
                            .min_size(Vec2::new(half_w, 26.0)),
                        )
                        .on_hover_text("View & customize keyboard shortcuts")
                        .clicked()
                    {
                        action = Some(SidebarAction::OpenShortcuts);
                    }
                });
            } else {
                ui.vertical_centered(|ui| {
                    if ui
                        .button(icon_rich(UiIcon::Plus, 13.0, None))
                        .on_hover_text("New Workspace")
                        .clicked()
                    {
                        action = Some(SidebarAction::CreateWorkspace);
                    }
                    if ui
                        .small_button(icon_rich(UiIcon::Settings, 13.0, None))
                        .on_hover_text("Settings")
                        .clicked()
                    {
                        action = Some(SidebarAction::OpenViewSettings);
                    }
                    if ui
                        .small_button(icon_rich(UiIcon::Keyboard, 13.0, None))
                        .on_hover_text("Keyboard Shortcuts")
                        .clicked()
                    {
                        action = Some(SidebarAction::OpenShortcuts);
                    }
                });
            }
        });
    });

    action
}

fn status_dot_color(status: Option<AgentStatus>, tokens: &UiTokens) -> Color32 {
    match status {
        Some(
            AgentStatus::Working
            | AgentStatus::Thinking
            | AgentStatus::RunningTool
            | AgentStatus::Done,
        ) => tokens.status_running_fg,
        Some(AgentStatus::WaitingForInput) => Color32::from_rgb(245, 158, 11),
        Some(AgentStatus::Failed) => Color32::from_rgb(239, 68, 68),
        Some(AgentStatus::Idle) | None => Color32::from_rgb(100, 116, 139),
    }
}
