//! Unit 7 UI components: Live `agy acp` Card Panel renderer and Multi-Agent Team Grid modal.

use crate::chrome_ui::{centered_modal_pos, modal_frame};
use agemax_agy::acp::LiveAcpSession;
use agemax_core::layout::PaneId;
use eframe::egui::{self, Color32, Key, Rect, Stroke, Vec2};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpPaneAction {
    Restart(PaneId),
    SwitchToTerminal(PaneId),
}

pub enum AcpPaneState {
    Running(Box<LiveAcpSession>),
    Failed(String),
}

/// Render an interactive visual card panel streaming a live `agy acp` stdio session (Unit 7 §D).
pub fn render_acp_pane(
    ui: &mut egui::Ui,
    pane_id: PaneId,
    body_rect: Rect,
    acp_state: &mut AcpPaneState,
) -> Option<AcpPaneAction> {
    let mut action = None;

    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("acp_panel", pane_id.0))
            .max_rect(body_rect.shrink(8.0)),
        |ui| match acp_state {
            AcpPaneState::Failed(err) => {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("🤖 Antigravity ACP Panel Unavailable")
                            .size(14.0)
                            .color(Color32::from_rgb(245, 175, 45))
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(err.as_str())
                            .size(11.5)
                            .color(Color32::from_rgb(210, 215, 225)),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("↻ Retry agy acp").clicked() {
                            action = Some(AcpPaneAction::Restart(pane_id));
                        }
                        if ui.button("← Back to Terminal").clicked() {
                            action = Some(AcpPaneAction::SwitchToTerminal(pane_id));
                        }
                    });
                });
            }
            AcpPaneState::Running(session) => {
                ui.vertical(|ui| {
                    // Header strip
                    ui.horizontal(|ui| {
                        let banner = session
                            .state
                            .server_banner
                            .as_deref()
                            .unwrap_or("Connecting agy acp…");
                        ui.label(
                            egui::RichText::new(format!("🤖 {banner}"))
                                .size(12.5)
                                .color(Color32::from_rgb(130, 195, 255))
                                .strong(),
                        );
                        if let Some(st) = &session.state.latest_status {
                            ui.label(
                                egui::RichText::new(format!("[{st}]"))
                                    .size(11.0)
                                    .color(Color32::from_rgb(125, 225, 165)),
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("← Terminal").clicked() {
                                action = Some(AcpPaneAction::SwitchToTerminal(pane_id));
                            }
                            if ui.small_button("↻ Restart").clicked() {
                                action = Some(AcpPaneAction::Restart(pane_id));
                            }
                        });
                    });

                    ui.separator();

                    // Active tools + artifacts strip
                    if !session.state.active_tools.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("Tools:")
                                    .size(11.0)
                                    .color(Color32::from_rgb(180, 185, 200))
                                    .strong(),
                            );
                            for t in &session.state.active_tools {
                                ui.label(
                                    egui::RichText::new(format!("⟳ {}", t.tool))
                                        .size(11.0)
                                        .color(Color32::from_rgb(245, 195, 85)),
                                );
                            }
                        });
                    }

                    if !session.state.recent_artifacts.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("Artifacts:")
                                    .size(11.0)
                                    .color(Color32::from_rgb(180, 185, 200))
                                    .strong(),
                            );
                            for art in session.state.recent_artifacts.iter().rev().take(4) {
                                ui.label(
                                    egui::RichText::new(format!("📄 {}", art.title))
                                        .size(11.0)
                                        .color(Color32::from_rgb(125, 215, 245)),
                                );
                            }
                        });
                    }

                    // Transcript output card
                    let scroll_h = (ui.available_height() - 42.0).max(80.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 21, 30))
                        .corner_radius(6.0)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(40, 46, 65)))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt(("acp_scroll", pane_id.0))
                                .max_height(scroll_h)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    let body = if session.state.latest_message.is_empty() {
                                        if session.state.session_id.is_some() {
                                            "Session ready. Enter a prompt below to converse with Antigravity over structured ACP stdio cards."
                                        } else {
                                            "Initializing ACP session..."
                                        }
                                    } else {
                                        session.state.latest_message.as_str()
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(body)
                                                .size(12.5)
                                                .color(Color32::from_rgb(225, 230, 242)),
                                        )
                                        .wrap(),
                                    );
                                });
                        });

                    ui.add_space(6.0);

                    // Prompt composer bar
                    ui.horizontal(|ui| {
                        let ready = session.state.session_id.is_some() && !session.state.is_turn_active;
                        let input_w = (ui.available_width() - 78.0).max(120.0);
                        let resp = ui.add_enabled(
                            ready,
                            egui::TextEdit::singleline(&mut session.prompt_draft)
                                .desired_width(input_w)
                                .hint_text("Ask Antigravity via ACP...")
                                .text_color(Color32::WHITE),
                        );
                        let send_clicked = ui
                            .add_enabled(ready, egui::Button::new("Send ↵"))
                            .clicked();
                        let enter_pressed =
                            resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                        if (send_clicked || enter_pressed) && ready {
                            let text = session.prompt_draft.clone();
                            session.prompt_draft.clear();
                            if let Err(e) = session.send_prompt(&text) {
                                session.state.latest_status = Some(e);
                            }
                        }
                    });
                });
            }
        },
    );

    action
}

/// Request returned by `TeamLauncherModal` to spawn a coordinated multi-agent layout tab.
#[derive(Debug, Clone)]
pub struct TeamLaunchRequest {
    pub coordinator_role: String,
    pub worker_roles: Vec<String>,
    pub launch_agy_in_workers: bool,
    pub use_acp_for_coordinator: bool,
}

#[derive(Debug)]
pub struct TeamLauncherModal {
    pub is_open: bool,
    worker_count: usize,
    launch_agy_in_workers: bool,
    use_acp_for_coordinator: bool,
}

impl Default for TeamLauncherModal {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamLauncherModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            worker_count: 2,
            launch_agy_in_workers: true,
            use_acp_for_coordinator: true,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<TeamLaunchRequest> {
        if !self.is_open {
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        let mut submitted = None;
        let mut should_close = false;
        let pos = centered_modal_pos(ctx, 470.0, 260.0);

        egui::Window::new("team_launcher_modal")
            .fixed_pos(pos)
            .fixed_size(Vec2::new(470.0, 250.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Launch Multi-Agent Team Grid")
                            .size(15.5)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Creates a coordinated split tab linking Coordinator and Worker agent panes.")
                            .size(11.5)
                            .color(Color32::from_rgb(160, 170, 190)),
                    );
                    ui.add_space(10.0);

                    ui.label("Team Size Preset:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.worker_count, 1, "Duo (1 Coord + 1 Worker)");
                        ui.selectable_value(&mut self.worker_count, 2, "Trio (1 Coord + 2 Workers)");
                        ui.selectable_value(&mut self.worker_count, 3, "Squad (1 Coord + 3 Workers)");
                    });

                    ui.add_space(10.0);
                    ui.checkbox(
                        &mut self.use_acp_for_coordinator,
                        "Use structured Antigravity ACP Card Panel for Coordinator",
                    );
                    ui.checkbox(
                        &mut self.launch_agy_in_workers,
                        "Start interactive agy session inside each Worker pane",
                    );

                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Launch Team Grid").clicked() {
                            let roles = match self.worker_count {
                                1 => vec!["Worker".to_string()],
                                2 => vec!["Researcher".to_string(), "Builder".to_string()],
                                _ => vec![
                                    "Researcher".to_string(),
                                    "Builder".to_string(),
                                    "Verifier".to_string(),
                                ],
                            };
                            submitted = Some(TeamLaunchRequest {
                                coordinator_role: "Coordinator".to_string(),
                                worker_roles: roles,
                                launch_agy_in_workers: self.launch_agy_in_workers,
                                use_acp_for_coordinator: self.use_acp_for_coordinator,
                            });
                            should_close = true;
                        }
                        if ui.button("Cancel").clicked() {
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

/// Helper to position subtle top badge strip for team panes when headers are hidden.
pub fn team_badge_color(is_coordinator: bool) -> Color32 {
    if is_coordinator {
        Color32::from_rgb(130, 165, 255)
    } else {
        Color32::from_rgb(115, 215, 185)
    }
}
