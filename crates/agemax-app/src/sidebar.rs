use crate::icons::{icon_label, icon_rich, icon_text, status_icon, UiIcon};
use crate::telemetry::{format_elapsed_compact, render_sparkline, SPARKLINE_BUCKETS};
use crate::ui_tokens::{metrics, UiTokens};
use crate::visual_alive::ThemeChromeColors;
use agemax_config::theme::ResolvedTheme;
use agemax_core::status::AgentStatus;
use agemax_core::workspace::Workspace;
use agemax_term::metadata::GitDiffStat;
use eframe::egui::{self, Color32, Stroke, Vec2};

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
    let chrome = ThemeChromeColors::from_theme(props.theme);
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
                            chrome.panel_alt_bg
                        } else if has_unread {
                            Color32::from_rgb(42, 34, 22)
                        } else {
                            Color32::TRANSPARENT
                        };

                        let btn_frame = egui::Frame::new()
                            .fill(bg_color)
                            .corner_radius(4.0)
                            .stroke(if is_active {
                                Stroke::new(1.0, chrome.accent)
                            } else {
                                Stroke::NONE
                            })
                            .inner_margin(6.0);

                        btn_frame.show(ui, |ui| {
                            if props.is_collapsed {
                                let first_letter =
                                    ws.name.chars().next().unwrap_or('W').to_string();
                                let text_color = if is_active {
                                    Color32::WHITE
                                } else if has_unread {
                                    Color32::from_rgb(245, 175, 45)
                                } else {
                                    Color32::GRAY
                                };
                                let resp = ui.button(
                                    egui::RichText::new(first_letter)
                                        .color(text_color)
                                        .size(14.0)
                                        .strong(),
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
                                ui.horizontal(|ui| {
                                    let label_color = if is_active {
                                        chrome.text_primary
                                    } else if has_unread {
                                        Color32::from_rgb(245, 175, 45)
                                    } else {
                                        chrome.text_muted
                                    };
                                    let label_text = egui::RichText::new(&ws.name)
                                        .color(label_color)
                                        .size(13.0)
                                        .strong();
                                    if ui.link(label_text).clicked() {
                                        action = Some(SidebarAction::SelectWorkspace(idx));
                                    }

                                    if let Some(status) = meta.status {
                                        ui.label(icon_rich(
                                            status_icon(status),
                                            11.0,
                                            Some(chrome.text_muted),
                                        ));
                                    }

                                    if let Some(elapsed) = meta.working_elapsed_secs {
                                        ui.label(
                                            egui::RichText::new(format_elapsed_compact(elapsed))
                                                .size(10.0)
                                                .color(chrome.accent)
                                                .strong(),
                                        );
                                    }

                                    if has_unread {
                                        ui.label(
                                            egui::RichText::new(format!("[{}]", meta.unread_count))
                                                .size(11.0)
                                                .color(Color32::from_rgb(245, 175, 45))
                                                .strong(),
                                        );
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if props.workspaces.len() > 1
                                                && ui
                                                    .small_button(icon_rich(UiIcon::X, 10.5, None))
                                                    .on_hover_text("Close workspace")
                                                    .clicked()
                                            {
                                                action = Some(SidebarAction::CloseWorkspace(idx));
                                            }
                                            if ui
                                                .small_button(icon_rich(UiIcon::Pencil, 10.5, None))
                                                .on_hover_text("Rename workspace")
                                                .clicked()
                                            {
                                                action = Some(SidebarAction::RenameWorkspace(idx));
                                            }
                                        },
                                    );
                                });

                                let folder_name = ws
                                    .cwd
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_else(|| ws.cwd.to_string_lossy().to_string());
                                ui.horizontal(|ui| {
                                    let subtitle = match ws.backend.short_badge() {
                                        Some(badge) => format!("[{badge}]"),
                                        None => folder_name,
                                    };
                                    ui.label(
                                        egui::RichText::new(subtitle)
                                            .size(10.0)
                                            .color(chrome.text_muted),
                                    );
                                    if let Some(branch) = &ws.git_branch {
                                        ui.label(icon_label(
                                            UiIcon::GitBranch,
                                            branch,
                                            10.0,
                                            Some(Color32::from_rgb(110, 190, 140)),
                                        ));
                                    }
                                    if let Some(stat) = meta.git_stats {
                                        if stat.has_changes() {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "+{}",
                                                    stat.insertions
                                                ))
                                                .size(10.0)
                                                .color(Color32::from_rgb(80, 215, 130)),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("-{}", stat.deletions))
                                                    .size(10.0)
                                                    .color(Color32::from_rgb(240, 105, 105)),
                                            );
                                        }
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (srect, _) = ui.allocate_exact_size(
                                                Vec2::new(42.0, 11.0),
                                                egui::Sense::hover(),
                                            );
                                            render_sparkline(
                                                ui.painter(),
                                                srect,
                                                &meta.sparkline,
                                                chrome.accent,
                                                chrome.panel_stroke,
                                            );
                                        },
                                    );
                                });

                                if let Some(snippet) = &meta.latest_snippet {
                                    ui.add(
                                        egui::Label::new(icon_label(
                                            UiIcon::AlertTriangle,
                                            snippet,
                                            10.0,
                                            Some(Color32::from_rgb(245, 175, 45)),
                                        ))
                                        .truncate(),
                                    );
                                } else if let Some(last_line) = &meta.last_output_line {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("> {last_line}"))
                                                .size(10.0)
                                                .monospace()
                                                .color(chrome.text_muted),
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
