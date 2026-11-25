//! Centralized visual design system tokens derived from `ResolvedTheme`.

use agemax_config::theme::ResolvedTheme;
use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Consistent UI spacing and corner radius constants across chrome panels.
pub mod metrics {
    use eframe::egui::Vec2;

    pub const RADIUS_SM: f32 = 6.0;
    pub const RADIUS_MD: f32 = 8.0;
    pub const RADIUS_PILL: f32 = 12.0;

    pub const HEADER_HEIGHT: f32 = 38.0;
    pub const TOOLBAR_HEIGHT: f32 = 40.0;
    pub const PATH_BAR_HEIGHT: f32 = 30.0;
    pub const STATUS_BAR_HEIGHT: f32 = 26.0;

    pub const ICON_BTN_SIZE: Vec2 = Vec2::new(28.0, 26.0);
}

/// Unified color tokens harmonized across all built-in theme presets.
#[derive(Debug, Clone, Copy)]
pub struct UiTokens {
    pub is_light: bool,
    pub bg_canvas: Color32,
    pub bg_chrome: Color32,
    pub bg_surface: Color32,
    pub bg_elevated: Color32,
    pub border_subtle: Color32,
    pub border_focus: Color32,
    pub accent_primary: Color32,
    pub accent_hover: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub status_running_fg: Color32,
    pub lock_fill: Color32,
    pub lock_border: Color32,
    pub lock_fg: Color32,
    pub unlocked_fill: Color32,
    pub unlocked_border: Color32,
    pub unlocked_fg: Color32,
    pub alert_badge_bg: Color32,
}

impl UiTokens {
    /// Derive all UI chrome tokens from a resolved theme palette.
    pub fn from_theme(theme: &ResolvedTheme) -> Self {
        let is_light = theme.is_light_background();
        let bg = theme.background;
        let fg = theme.foreground;
        let accent = theme.active_border;

        let shift = |base: (u8, u8, u8), delta: i16| -> Color32 {
            let adj = |v: u8| -> u8 {
                if is_light {
                    (v as i16 - delta).clamp(0, 255) as u8
                } else {
                    (v as i16 + delta).clamp(0, 255) as u8
                }
            };
            Color32::from_rgb(adj(base.0), adj(base.1), adj(base.2))
        };

        let accent_primary = Color32::from_rgb(
            (accent.0 as f32 * 0.88).round() as u8,
            (accent.1 as f32 * 0.72).round() as u8,
            (accent.2 as f32 * 0.96).round() as u8,
        );

        Self {
            is_light,
            bg_canvas: Color32::from_rgb(bg.0, bg.1, bg.2),
            bg_chrome: shift(bg, 5),
            bg_surface: shift(bg, 11),
            bg_elevated: shift(bg, 19),
            border_subtle: Color32::from_rgb(
                theme.inactive_border.0,
                theme.inactive_border.1,
                theme.inactive_border.2,
            ),
            border_focus: Color32::from_rgb(accent.0, accent.1, accent.2),
            accent_primary,
            accent_hover: Color32::from_rgb(accent.0, accent.1, accent.2),
            text_primary: Color32::from_rgb(fg.0, fg.1, fg.2),
            text_muted: Color32::from_rgb(
                ((fg.0 as u16 * 55 + bg.0 as u16 * 45) / 100) as u8,
                ((fg.1 as u16 * 55 + bg.1 as u16 * 45) / 100) as u8,
                ((fg.2 as u16 * 55 + bg.2 as u16 * 45) / 100) as u8,
            ),
            status_running_fg: Color32::from_rgb(74, 222, 128),
            lock_fill: if is_light {
                Color32::from_rgb(220, 248, 234)
            } else {
                Color32::from_rgb(14, 43, 34)
            },
            lock_border: if is_light {
                Color32::from_rgb(34, 139, 94)
            } else {
                Color32::from_rgb(31, 110, 78)
            },
            lock_fg: if is_light {
                Color32::from_rgb(15, 118, 70)
            } else {
                Color32::from_rgb(52, 211, 153)
            },
            unlocked_fill: if is_light {
                Color32::from_rgb(254, 243, 199)
            } else {
                Color32::from_rgb(44, 32, 14)
            },
            unlocked_border: if is_light {
                Color32::from_rgb(217, 119, 6)
            } else {
                Color32::from_rgb(138, 92, 24)
            },
            unlocked_fg: if is_light {
                Color32::from_rgb(180, 83, 9)
            } else {
                Color32::from_rgb(251, 191, 36)
            },
            alert_badge_bg: Color32::from_rgb(239, 68, 68),
        }
    }

    /// Apply consistent button, stroke, and background styling to `egui::Context`.
    pub fn apply_visuals(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };

        visuals.panel_fill = self.bg_chrome;
        visuals.window_fill = self.bg_surface;
        visuals.extreme_bg_color = self.bg_canvas;
        visuals.faint_bg_color = self.bg_surface;
        visuals.selection.bg_fill = self.accent_primary;
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

        let cr = CornerRadius::same(metrics::RADIUS_SM as u8);
        visuals.widgets.noninteractive.bg_fill = self.bg_chrome;
        visuals.widgets.noninteractive.weak_bg_fill = self.bg_chrome;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_muted);
        visuals.widgets.noninteractive.corner_radius = cr;

        visuals.widgets.inactive.bg_fill = self.bg_surface;
        visuals.widgets.inactive.weak_bg_fill = self.bg_surface;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.widgets.inactive.corner_radius = cr;

        visuals.widgets.hovered.bg_fill = self.bg_elevated;
        visuals.widgets.hovered.weak_bg_fill = self.bg_elevated;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.border_focus);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.widgets.hovered.corner_radius = cr;

        visuals.widgets.active.bg_fill = self.accent_primary;
        visuals.widgets.active.weak_bg_fill = self.accent_primary;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent_hover);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.active.corner_radius = cr;

        ctx.set_visuals(visuals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agemax_config::theme::{ThemeConfig, ThemePreset};

    #[test]
    fn test_ui_tokens_support_all_presets() {
        for preset in ThemePreset::all() {
            let resolved = ThemeConfig::from_preset(*preset).resolved_rgb();
            let tokens = UiTokens::from_theme(&resolved);
            assert_eq!(tokens.is_light, resolved.is_light_background());
            assert_ne!(tokens.bg_canvas, tokens.text_primary);
        }
    }
}
