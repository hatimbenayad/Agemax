use crate::telemetry::{format_elapsed_compact, render_sparkline, SPARKLINE_BUCKETS};
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
}

pub fn render_sidebar(ui: &mut egui::Ui, props: SidebarProps) -> Option<SidebarAction> {
    let mut action = None;
    let width = if props.is_collapsed { 48.0 } else { 220.0 };
    let chrome = ThemeChromeColors::from_theme(props.theme);

    ui.allocate_ui(Vec2::new(width, ui.available_height()), |ui| {
        let frame = egui::Frame::new()
            .fill(chrome.panel_bg)
            .stroke(Stroke::new(1.0, chrome.panel_stroke));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if !props.is_collapsed {
                        ui.heading(
                            egui::RichText::new("WORKSPACES")
                                .size(11.0)
                                .color(chrome.text_muted)
                                .strong(),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let toggle_label = if props.is_collapsed { "»" } else { "«" };
                        if ui.button(toggle_label).clicked() {
                            action = Some(SidebarAction::ToggleCollapse);
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Workspace list
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
                            let first_letter = ws.name.chars().next().unwrap_or('W').to_string();
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
                                    ui.label(
                                        egui::RichText::new(status.icon())
                                            .size(11.0)
                                            .color(chrome.text_muted),
                                    );
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
                                                .small_button("×")
                                                .on_hover_text("Close workspace")
                                                .clicked()
                                        {
                                            action = Some(SidebarAction::CloseWorkspace(idx));
                                        }
                                        if ui
                                            .small_button("✎")
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
                                    ui.label(
                                        egui::RichText::new(format!("⎇ {branch}"))
                                            .size(10.0)
                                            .color(Color32::from_rgb(110, 190, 140)),
                                    );
                                }
                                if let Some(stat) = meta.git_stats {
                                    if stat.has_changes() {
                                        ui.label(
                                            egui::RichText::new(format!("+{}", stat.insertions))
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
                                    egui::Label::new(
                                        egui::RichText::new(format!("⚠ {}", snippet))
                                            .size(10.0)
                                            .color(Color32::from_rgb(245, 175, 45)),
                                    )
                                    .truncate(),
                                );
                            } else if let Some(last_line) = &meta.last_output_line {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("› {}", last_line))
                                            .size(10.0)
                                            .monospace()
                                            .color(chrome.text_muted),
                                    )
                                    .truncate(),
                                );
                            }
                        }
                    });

                    ui.add_space(2.0);
                }

                // Add Workspace & View Settings controls
                ui.add_space(8.0);
                if !props.is_collapsed {
                    ui.horizontal(|ui| {
                        if ui.button("+ New Workspace").clicked() {
                            action = Some(SidebarAction::CreateWorkspace);
                        }
                    });
                    ui.add_space(6.0);
                    if ui
                        .small_button("⚙ View & Shortcuts")
                        .on_hover_text("Switch Guided/Minimal mode or inspect shortcuts")
                        .clicked()
                    {
                        action = Some(SidebarAction::OpenViewSettings);
                    }
                } else {
                    if ui.button("+").on_hover_text("New Workspace").clicked() {
                        action = Some(SidebarAction::CreateWorkspace);
                    }
                    if ui
                        .small_button("⚙")
                        .on_hover_text("View & Shortcuts")
                        .clicked()
                    {
                        action = Some(SidebarAction::OpenViewSettings);
                    }
                }
            });
        });
    });

    action
}
