//! Unit 13b Mission Control UI components (`REQ-088`):
//! - All-Agents Overview Grid View
//! - Chronological Global Activity Feed + Unread Alerts Drawer
//! - Safe Target-Confirmed Broadcast Prompt Modal
//! - Welcome / Quick-Launch Empty State Card View

use crate::chrome_ui::{centered_modal_pos, modal_frame};
use crate::telemetry::{format_elapsed_compact, render_sparkline, SPARKLINE_BUCKETS};
use crate::visual_alive::{render_agent_mascot, ThemeChromeColors};
use agemax_config::theme::ResolvedTheme;
use agemax_core::layout::{PaneId, WorkspaceId};
use agemax_core::notification::{ActivityFeedEntry, NotificationItem};
use agemax_core::session::SessionSnapshot;
use agemax_core::status::AgentStatus;
use agemax_term::metadata::GitDiffStat;
use eframe::egui::{self, Color32, Key, Stroke, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;

/// Tab selector inside the right-hand Mission Control drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightDrawerTab {
    #[default]
    Alerts,
    ActivityFeed,
}

#[derive(Debug, Clone)]
pub struct GridPaneCard {
    pub workspace_index: usize,
    pub workspace_name: String,
    pub tab_index: usize,
    pub tab_title: String,
    pub pane_id: PaneId,
    pub status: AgentStatus,
    pub is_focused: bool,
    pub git_branch: Option<String>,
    pub git_stats: Option<GitDiffStat>,
    pub last_line: Option<String>,
    pub elapsed_secs: Option<u64>,
    pub sparkline: [u32; SPARKLINE_BUCKETS],
    pub kind_label: &'static str,
}

pub enum GridViewAction {
    FocusPane {
        workspace_index: usize,
        tab_index: usize,
        pane_id: PaneId,
    },
    ExitGridView,
}

/// Render responsive All-Agents Grid View showing live status, mascots, sparklines, and output (`REQ-088`).
pub fn render_agents_grid(
    ui: &mut egui::Ui,
    cards: &[GridPaneCard],
    resolved_theme: &ResolvedTheme,
) -> Option<GridViewAction> {
    let mut action = None;
    let chrome = ThemeChromeColors::from_theme(resolved_theme);
    let time_secs = ui.input(|i| i.time);

    ui.vertical(|ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⊞ ALL AGENTS MISSION GRID")
                    .size(13.0)
                    .color(chrome.text_primary)
                    .strong(),
            );
            ui.label(
                egui::RichText::new(format!("({} live sessions)", cards.len()))
                    .size(11.5)
                    .color(chrome.text_muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("← Return to Split View").clicked() {
                    action = Some(GridViewAction::ExitGridView);
                }
            });
        });
        ui.separator();
        ui.add_space(6.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let avail_w = ui.available_width().max(260.0);
            let cols = if avail_w >= 860.0 {
                3
            } else if avail_w >= 540.0 {
                2
            } else {
                1
            };
            let spacing = 10.0;
            let card_w = ((avail_w - spacing * (cols as f32 - 1.0)) / cols as f32).max(220.0);

            for row_chunk in cards.chunks(cols) {
                ui.horizontal(|ui| {
                    for card in row_chunk {
                        let border_col = status_color(card.status, resolved_theme);
                        let frame = egui::Frame::new()
                            .fill(chrome.panel_bg)
                            .corner_radius(6.0)
                            .stroke(Stroke::new(
                                if card.is_focused { 2.0 } else { 1.2 },
                                border_col,
                            ))
                            .inner_margin(10.0);

                        ui.allocate_ui(Vec2::new(card_w, 138.0), |ui| {
                            frame.show(ui, |ui| {
                                ui.set_min_size(Vec2::new(card_w - 20.0, 118.0));
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        let (mrect, _) = ui.allocate_exact_size(
                                            Vec2::new(22.0, 20.0),
                                            egui::Sense::hover(),
                                        );
                                        render_agent_mascot(
                                            ui.painter(),
                                            mrect,
                                            card.status,
                                            time_secs,
                                            resolved_theme,
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} · {}",
                                                card.workspace_name, card.tab_title
                                            ))
                                            .size(12.0)
                                            .color(chrome.text_primary)
                                            .strong(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("Open").clicked() {
                                                    action = Some(GridViewAction::FocusPane {
                                                        workspace_index: card.workspace_index,
                                                        tab_index: card.tab_index,
                                                        pane_id: card.pane_id,
                                                    });
                                                }
                                            },
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} {}",
                                                card.status.icon(),
                                                card.status.display_name()
                                            ))
                                            .size(11.0)
                                            .color(border_col)
                                            .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("• {}", card.kind_label))
                                                .size(10.5)
                                                .color(chrome.text_muted),
                                        );
                                        if let Some(secs) = card.elapsed_secs {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "• {}",
                                                    format_elapsed_compact(secs)
                                                ))
                                                .size(10.5)
                                                .color(chrome.text_muted),
                                            );
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        if let Some(branch) = &card.git_branch {
                                            ui.label(
                                                egui::RichText::new(format!("⎇ {branch}"))
                                                    .size(10.5)
                                                    .color(Color32::from_rgb(110, 195, 145)),
                                            );
                                        }
                                        if let Some(stat) = card.git_stats {
                                            if stat.has_changes() {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "+{}",
                                                        stat.insertions
                                                    ))
                                                    .size(10.5)
                                                    .color(Color32::from_rgb(80, 215, 130)),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "-{}",
                                                        stat.deletions
                                                    ))
                                                    .size(10.5)
                                                    .color(Color32::from_rgb(240, 105, 105)),
                                                );
                                            }
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let (srect, _) = ui.allocate_exact_size(
                                                    Vec2::new(64.0, 14.0),
                                                    egui::Sense::hover(),
                                                );
                                                render_sparkline(
                                                    ui.painter(),
                                                    srect,
                                                    &card.sparkline,
                                                    chrome.accent,
                                                    chrome.panel_stroke,
                                                );
                                            },
                                        );
                                    });

                                    ui.add_space(4.0);
                                    let snippet = card
                                        .last_line
                                        .as_deref()
                                        .unwrap_or("Waiting for terminal activity...");
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(snippet)
                                                .size(10.5)
                                                .monospace()
                                                .color(chrome.text_muted),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                        });
                    }
                });
                ui.add_space(spacing);
            }
        });
    });

    action
}

pub enum RightDrawerAction {
    SwitchTab(RightDrawerTab),
    FocusPane(WorkspaceId, PaneId),
    ClearFeed,
}

/// Render combined Alerts & Global Activity Feed panel (`REQ-088`).
pub fn render_right_drawer(
    ui: &mut egui::Ui,
    active_tab: RightDrawerTab,
    unread: &[&NotificationItem],
    feed_entries: &[&ActivityFeedEntry],
    resolved_theme: &ResolvedTheme,
) -> Option<RightDrawerAction> {
    let mut action = None;
    let chrome = ThemeChromeColors::from_theme(resolved_theme);

    ui.allocate_ui(Vec2::new(272.0, ui.available_height()), |ui| {
        let frame = egui::Frame::new()
            .fill(chrome.panel_bg)
            .stroke(Stroke::new(1.0, chrome.panel_stroke))
            .inner_margin(8.0);

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        active_tab == RightDrawerTab::Alerts,
                        format!("🔔 Alerts ({})", unread.len()),
                    )
                    .clicked()
                {
                    action = Some(RightDrawerAction::SwitchTab(RightDrawerTab::Alerts));
                }
                if ui
                    .selectable_label(
                        active_tab == RightDrawerTab::ActivityFeed,
                        format!("📡 Feed ({})", feed_entries.len()),
                    )
                    .clicked()
                {
                    action = Some(RightDrawerAction::SwitchTab(RightDrawerTab::ActivityFeed));
                }
                if active_tab == RightDrawerTab::ActivityFeed && !feed_entries.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Clear").clicked() {
                            action = Some(RightDrawerAction::ClearFeed);
                        }
                    });
                }
            });
            ui.separator();
            ui.add_space(4.0);

            egui::ScrollArea::vertical().show(ui, |ui| match active_tab {
                RightDrawerTab::Alerts => {
                    if unread.is_empty() {
                        ui.label(
                            egui::RichText::new("All caught up! No unread notifications.")
                                .size(11.0)
                                .color(chrome.text_muted),
                        );
                    } else {
                        for item in unread {
                            let col = status_color(item.status, resolved_theme);
                            ui.group(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} {}",
                                        item.status.icon(),
                                        item.status.display_name()
                                    ))
                                    .size(11.0)
                                    .color(col)
                                    .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(&item.message)
                                        .size(11.0)
                                        .color(chrome.text_primary),
                                );
                                if ui.small_button("Focus Pane").clicked() {
                                    action = Some(RightDrawerAction::FocusPane(
                                        item.workspace_id,
                                        item.pane_id,
                                    ));
                                }
                            });
                            ui.add_space(4.0);
                        }
                    }
                }
                RightDrawerTab::ActivityFeed => {
                    if feed_entries.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "Live milestones across your workspaces (tools, git changes, finished tasks) appear here.",
                            )
                            .size(11.0)
                            .color(chrome.text_muted),
                        );
                    } else {
                        for entry in feed_entries {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&entry.workspace_name)
                                            .size(11.0)
                                            .color(chrome.accent)
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("[{}]", entry.category))
                                            .size(10.0)
                                            .color(chrome.text_muted),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("Go").clicked() {
                                                action = Some(RightDrawerAction::FocusPane(
                                                    entry.workspace_id,
                                                    entry.pane_id,
                                                ));
                                            }
                                        },
                                    );
                                });
                                ui.label(
                                    egui::RichText::new(&entry.summary)
                                        .size(11.0)
                                        .color(chrome.text_primary),
                                );
                            });
                            ui.add_space(3.0);
                        }
                    }
                }
            });
        });
    });

    action
}

#[derive(Debug, Clone)]
pub struct BroadcastTarget {
    pub pane_id: PaneId,
    pub workspace_name: String,
    pub tab_title: String,
    pub folder_display: String,
    pub backend_label: String,
    pub is_ready: bool,
    pub status: AgentStatus,
}

#[derive(Debug, Clone)]
pub struct BroadcastSubmitRequest {
    pub prompt: String,
    pub append_enter: bool,
    pub target_panes: Vec<PaneId>,
}

/// Safe prompt broadcast dialog requiring explicit target pane confirmation (`REQ-088`).
#[derive(Debug, Default)]
pub struct BroadcastModal {
    pub is_open: bool,
    prompt_text: String,
    append_enter: bool,
    selected_panes: HashSet<PaneId>,
}

impl BroadcastModal {
    pub fn open(&mut self, targets: &[BroadcastTarget]) {
        self.is_open = true;
        self.append_enter = true;
        self.selected_panes.clear();
        for t in targets {
            if t.is_ready && t.backend_label == "Local" {
                self.selected_panes.insert(t.pane_id);
            }
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        targets: &[BroadcastTarget],
    ) -> Option<BroadcastSubmitRequest> {
        if !self.is_open {
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        let mut submitted = None;
        let mut should_close = false;
        let pos = centered_modal_pos(ctx, 520.0, 400.0);

        egui::Window::new("broadcast_prompt_modal")
            .fixed_pos(pos)
            .fixed_size(Vec2::new(520.0, 390.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📣 Safe Prompt Broadcast")
                                .size(15.0)
                                .color(Color32::WHITE)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Cancel").clicked() {
                                should_close = true;
                            }
                        });
                    });
                    ui.label(
                        egui::RichText::new(
                            "Verify target folders before sending. Exited panes, browsers, and unchecked remote shells are skipped.",
                        )
                        .size(11.0)
                        .color(Color32::from_rgb(165, 175, 195)),
                    );
                    ui.add_space(6.0);

                    ui.add(
                        egui::TextEdit::multiline(&mut self.prompt_text)
                            .desired_rows(3)
                            .desired_width(490.0)
                            .hint_text("Type a prompt or command to send to selected sessions...")
                            .text_color(Color32::WHITE),
                    );
                    ui.checkbox(&mut self.append_enter, "Append Enter (submit immediately)");

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Select Target Sessions:")
                            .size(12.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(145.0)
                        .show(ui, |ui| {
                            for target in targets {
                                ui.horizontal(|ui| {
                                    let mut checked =
                                        target.is_ready && self.selected_panes.contains(&target.pane_id);
                                    if ui
                                        .add_enabled(
                                            target.is_ready,
                                            egui::Checkbox::new(&mut checked, ""),
                                        )
                                        .changed()
                                    {
                                        if checked {
                                            self.selected_panes.insert(target.pane_id);
                                        } else {
                                            self.selected_panes.remove(&target.pane_id);
                                        }
                                    }
                                    let label = format!(
                                        "{} {} ({}) • [{}] • {}",
                                        target.status.icon(),
                                        target.workspace_name,
                                        target.tab_title,
                                        target.backend_label,
                                        target.folder_display
                                    );
                                    ui.label(
                                        egui::RichText::new(label)
                                            .size(11.0)
                                            .color(if target.is_ready {
                                                Color32::from_rgb(215, 222, 238)
                                            } else {
                                                Color32::GRAY
                                            }),
                                    );
                                });
                            }
                        });

                    ui.add_space(8.0);
                    let chosen: Vec<PaneId> = targets
                        .iter()
                        .filter(|t| t.is_ready && self.selected_panes.contains(&t.pane_id))
                        .map(|t| t.pane_id)
                        .collect();
                    let can_send = !self.prompt_text.trim().is_empty() && !chosen.is_empty();
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                can_send,
                                egui::Button::new(format!(
                                    "Send to {} Selected Session{}",
                                    chosen.len(),
                                    if chosen.len() == 1 { "" } else { "s" }
                                )),
                            )
                            .clicked()
                        {
                            submitted = Some(BroadcastSubmitRequest {
                                prompt: self.prompt_text.clone(),
                                append_enter: self.append_enter,
                                target_panes: chosen,
                            });
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.close();
        }
        submitted
    }
}

pub enum WelcomeAction {
    LaunchNewAgent,
    LaunchSafePlanAgent,
    LaunchTeamGrid,
    OpenNewWorkspaceModal,
    RestoreRecentWorkspace(PathBuf),
    Dismiss,
}

const ASCII_EMBLEM: &str = "\
   ___    ____ ______   __  ___ ___   _  __
  / _ |  / __// ____/  /  |/  // _ | | |/_/
 / __ | / _ \\/ /__    / /|_/ // __ |_>  <  
/_/ |_| \\___/\\___/   /_/  /_//_/ |_/_/|_|";

/// Render clean-room Welcome / Quick-Launch Empty State (`REQ-088`).
pub fn render_welcome_view(
    ui: &mut egui::Ui,
    recent_snapshot: Option<&SessionSnapshot>,
    active_cwd: &std::path::Path,
    resolved_theme: &ResolvedTheme,
) -> Option<WelcomeAction> {
    let mut action = None;
    let chrome = ThemeChromeColors::from_theme(resolved_theme);

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(18.0);
            ui.label(
                egui::RichText::new(ASCII_EMBLEM)
                    .monospace()
                    .size(13.0)
                    .color(chrome.accent),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Native Windows Agent Mission Control")
                    .size(15.0)
                    .color(chrome.text_primary)
                    .strong(),
            );
            ui.label(
                egui::RichText::new(format!("Current workspace: {}", active_cwd.display()))
                    .size(11.5)
                    .color(chrome.text_muted),
            );
            ui.add_space(14.0);

            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 560.0).max(16.0) * 0.5);
                for (title, sub, act) in [
                    ("▶ Start Agent", "Open interactive agy", 1u8),
                    ("🛡 Plan Agent", "Run agy --mode plan", 2u8),
                    ("👥 Agent Team Grid", "Coordinator + Workers", 3u8),
                ] {
                    let frame = egui::Frame::new()
                        .fill(chrome.panel_bg)
                        .corner_radius(6.0)
                        .stroke(Stroke::new(1.0, chrome.panel_stroke))
                        .inner_margin(10.0);
                    frame.show(ui, |ui| {
                        ui.set_min_width(168.0);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(title)
                                    .size(12.5)
                                    .color(chrome.text_primary)
                                    .strong(),
                            );
                            ui.label(egui::RichText::new(sub).size(11.0).color(chrome.text_muted));
                            ui.add_space(6.0);
                            if ui.button("Launch").clicked() {
                                action = Some(match act {
                                    1 => WelcomeAction::LaunchNewAgent,
                                    2 => WelcomeAction::LaunchSafePlanAgent,
                                    _ => WelcomeAction::LaunchTeamGrid,
                                });
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            });

            ui.add_space(14.0);
            if let Some(snap) = recent_snapshot {
                if !snap.workspaces.is_empty() {
                    ui.label(
                        egui::RichText::new("Recent Workspaces")
                            .size(12.5)
                            .color(chrome.text_primary)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    for ws in snap.workspaces.iter().take(4) {
                        if ui
                            .button(format!("📁 {} — {}", ws.name, ws.cwd.display()))
                            .clicked()
                        {
                            action = Some(WelcomeAction::RestoreRecentWorkspace(ws.cwd.clone()));
                        }
                    }
                    ui.add_space(10.0);
                }
            }

            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 290.0).max(16.0) * 0.5);
                if ui.button("+ Pick Project Folder…").clicked() {
                    action = Some(WelcomeAction::OpenNewWorkspaceModal);
                }
                if ui.button("Go Straight to Terminal →").clicked() {
                    action = Some(WelcomeAction::Dismiss);
                }
            });
        });
    });

    action
}

pub fn status_color(status: AgentStatus, theme: &ResolvedTheme) -> Color32 {
    match status {
        AgentStatus::Idle => Color32::from_rgb(
            theme.inactive_border.0.saturating_add(55),
            theme.inactive_border.1.saturating_add(55),
            theme.inactive_border.2.saturating_add(65),
        ),
        AgentStatus::Working | AgentStatus::Thinking | AgentStatus::RunningTool => {
            Color32::from_rgb(
                theme.active_border.0,
                theme.active_border.1,
                theme.active_border.2,
            )
        }
        AgentStatus::WaitingForInput => Color32::from_rgb(
            theme.ring_waiting.0,
            theme.ring_waiting.1,
            theme.ring_waiting.2,
        ),
        AgentStatus::Done => {
            Color32::from_rgb(theme.ring_done.0, theme.ring_done.1, theme.ring_done.2)
        }
        AgentStatus::Failed => Color32::from_rgb(
            theme.ring_failed.0,
            theme.ring_failed.1,
            theme.ring_failed.2,
        ),
    }
}
