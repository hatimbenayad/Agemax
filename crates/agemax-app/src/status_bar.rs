//! Bottom application status bar (`Panel::bottom`).

use crate::icons::{icon_rich, UiIcon};
use crate::ui_tokens::{metrics, UiTokens};
use agemax_config::theme::ResolvedTheme;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

pub struct StatusBarProps<'a> {
    pub git_branch: Option<&'a str>,
    pub folder_name: &'a str,
    pub running_agents: usize,
    pub cpu_percent: Option<u8>,
    pub ram_percent: Option<u8>,
    pub font_size: f32,
    pub command_palette_shortcut: &'a str,
    pub resolved_theme: &'a ResolvedTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarAction {
    OpenCommandPalette,
}

pub fn render_status_bar(ui: &mut egui::Ui, props: StatusBarProps<'_>) -> Option<StatusBarAction> {
    let tokens = UiTokens::from_theme(props.resolved_theme);
    let mut action = None;

    let frame = egui::Frame::new()
        .fill(tokens.bg_chrome)
        .inner_margin(Vec2::new(10.0, 4.0));

    let resp = frame.show(ui, |ui| {
        ui.set_min_height(metrics::STATUS_BAR_HEIGHT - 8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;

            if let Some(branch) = props.git_branch {
                ui.label(icon_rich(UiIcon::GitBranch, 11.5, Some(tokens.text_muted)));
                ui.label(
                    egui::RichText::new(branch)
                        .size(11.0)
                        .color(tokens.text_primary),
                );
                status_divider(ui, &tokens);
            }

            ui.label(icon_rich(UiIcon::Folder, 11.5, Some(tokens.text_muted)));
            ui.label(
                egui::RichText::new(props.folder_name)
                    .size(11.0)
                    .color(tokens.text_muted),
            );
            status_divider(ui, &tokens);

            let dot_color = if props.running_agents > 0 {
                tokens.status_running_fg
            } else {
                tokens.text_muted
            };
            let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 12.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 3.5, dot_color);
            ui.label(
                egui::RichText::new(format_running_agents_label(props.running_agents))
                    .size(11.0)
                    .color(if props.running_agents > 0 {
                        tokens.text_primary
                    } else {
                        tokens.text_muted
                    }),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 7.0;

                let shortcut = props.command_palette_shortcut.trim();
                if !shortcut.is_empty() {
                    let shortcut_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(shortcut)
                                .size(10.5)
                                .color(tokens.text_muted),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(0.0, 16.0)),
                    );
                    if shortcut_btn.on_hover_text("Open Command Palette").clicked() {
                        action = Some(StatusBarAction::OpenCommandPalette);
                    }
                    status_divider(ui, &tokens);
                }

                ui.label(
                    egui::RichText::new(format!("{:.0}px", props.font_size))
                        .size(11.0)
                        .color(tokens.text_muted),
                );
                status_divider(ui, &tokens);

                ui.label(
                    egui::RichText::new("UTF-8")
                        .size(11.0)
                        .color(tokens.text_muted),
                );

                if let Some(ram) = props.ram_percent {
                    status_divider(ui, &tokens);
                    render_meter_pill(ui, &tokens, UiIcon::MemoryStick, "RAM", ram);
                }

                if let Some(cpu) = props.cpu_percent {
                    status_divider(ui, &tokens);
                    render_meter_pill(ui, &tokens, UiIcon::Cpu, "CPU", cpu);
                }
            });
        });
    });

    let outer = resp.response.rect;
    ui.painter().line_segment(
        [
            Pos2::new(outer.min.x, outer.min.y),
            Pos2::new(outer.max.x, outer.min.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );

    action
}

pub fn format_running_agents_label(count: usize) -> String {
    if count == 1 {
        "1 agent running".to_string()
    } else {
        format!("{count} agents running")
    }
}

fn render_meter_pill(ui: &mut egui::Ui, tokens: &UiTokens, icon: UiIcon, label: &str, percent: u8) {
    let clamped = percent.min(100);
    // right_to_left container reverses child insertion order, so emit rightmost elements first.
    ui.label(
        egui::RichText::new(format!("{clamped}%"))
            .size(11.0)
            .color(tokens.text_primary),
    );
    let (track_rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 5.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(track_rect, 2.5, tokens.bg_elevated);
    let fill_w = (track_rect.width() * (clamped as f32 / 100.0)).max(2.0);
    let fill_rect = Rect::from_min_size(track_rect.min, Vec2::new(fill_w, track_rect.height()));
    ui.painter()
        .rect_filled(fill_rect, 2.5, tokens.accent_primary);
    ui.label(
        egui::RichText::new(label)
            .size(11.0)
            .color(tokens.text_muted),
    );
    ui.label(icon_rich(icon, 11.5, Some(tokens.text_muted)));
}

fn status_divider(ui: &mut egui::Ui, tokens: &UiTokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 12.0), egui::Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(rect.center().x, rect.min.y),
            Pos2::new(rect.center().x, rect.max.y),
        ],
        Stroke::new(1.0, tokens.border_subtle),
    );
}

#[cfg(test)]
mod tests {
    use super::format_running_agents_label;

    #[test]
    fn test_running_agents_label_pluralization() {
        assert_eq!(format_running_agents_label(0), "0 agents running");
        assert_eq!(format_running_agents_label(1), "1 agent running");
        assert_eq!(format_running_agents_label(4), "4 agents running");
    }
}
