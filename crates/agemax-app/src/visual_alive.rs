use agemax_config::theme::ResolvedTheme;
use agemax_core::layout::PaneId;
use agemax_core::status::AgentStatus;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;
use std::time::Instant;

const COLOR_EASE_SECS: f32 = 0.20;
const PANE_RECT_EASE_SECS: f32 = 0.14;
const CONPTY_RESIZE_DEBOUNCE_SECS: f32 = 0.12;
const DONE_FLASH_SECS: f32 = 1.25;
const FAILED_PULSE_SECS: f32 = 1.00;

/// Cohesive chrome panel colors derived cleanly from `ResolvedTheme`.
#[derive(Debug, Clone, Copy)]
pub struct ThemeChromeColors {
    pub window_bg: Color32,
    pub panel_bg: Color32,
    pub panel_alt_bg: Color32,
    pub panel_stroke: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
}

impl ThemeChromeColors {
    /// Derive harmonizing UI surface tones from any resolved palette preset.
    pub fn from_theme(theme: &ResolvedTheme) -> Self {
        let bg = theme.background;
        let fg = theme.foreground;
        let accent = theme.active_border;
        let is_light = theme.is_light_background();
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

        Self {
            window_bg: Color32::from_rgb(bg.0, bg.1, bg.2),
            panel_bg: shift(bg, 7),
            panel_alt_bg: shift(bg, 14),
            panel_stroke: Color32::from_rgb(
                theme.inactive_border.0,
                theme.inactive_border.1,
                theme.inactive_border.2,
            ),
            text_primary: Color32::from_rgb(fg.0, fg.1, fg.2),
            text_muted: Color32::from_rgb(
                ((fg.0 as u16 + bg.0 as u16) / 2) as u8,
                ((fg.1 as u16 + bg.1 as u16) / 2) as u8,
                ((fg.2 as u16 + bg.2 as u16) / 2) as u8,
            ),
            accent: Color32::from_rgb(accent.0, accent.1, accent.2),
        }
    }
}

#[derive(Debug, Clone)]
struct PaneColorAnimState {
    start_rgb: [f32; 3],
    current_rgb: [f32; 3],
    target_rgb: [f32; 3],
    color_started_at: Instant,
    last_status: AgentStatus,
    state_entered_at: Instant,
}

/// Output styling for one frame of a pane's state-colored border and glow.
#[derive(Debug, Clone, Copy)]
pub struct PaneBorderVisual {
    pub border_color: Color32,
    pub border_width: f32,
    pub outer_glow_color: Option<Color32>,
    pub outer_glow_width: f32,
    pub focus_inner_color: Option<Color32>,
    pub animating: bool,
}

/// Smooth ~200 ms RGB color interpolator and per-state pulse coordinator (`REQ-084`).
/// Stops requesting active animation frames once all panes settle to `Idle`.
#[derive(Debug, Default)]
pub struct PaneVisualAnimator {
    states: HashMap<PaneId, PaneColorAnimState>,
}

impl PaneVisualAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove_pane(&mut self, pane_id: PaneId) {
        self.states.remove(&pane_id);
    }

    pub fn compute(
        &mut self,
        pane_id: PaneId,
        status: AgentStatus,
        is_focused: bool,
        team_color: Option<(u8, u8, u8)>,
        theme: &ResolvedTheme,
        now: Instant,
    ) -> PaneBorderVisual {
        let target_tuple = target_rgb_for_state(status, is_focused, team_color, theme);
        let target = [
            target_tuple.0 as f32,
            target_tuple.1 as f32,
            target_tuple.2 as f32,
        ];

        let entry = self
            .states
            .entry(pane_id)
            .or_insert_with(|| PaneColorAnimState {
                start_rgb: target,
                current_rgb: target,
                target_rgb: target,
                color_started_at: now,
                last_status: status,
                state_entered_at: now,
            });

        if entry.last_status != status {
            entry.last_status = status;
            entry.state_entered_at = now;
        }
        if entry.target_rgb != target {
            entry.start_rgb = entry.current_rgb;
            entry.target_rgb = target;
            entry.color_started_at = now;
        }

        let elapsed_color = now
            .saturating_duration_since(entry.color_started_at)
            .as_secs_f32();
        let raw_t = (elapsed_color / COLOR_EASE_SECS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - raw_t).powi(3);
        let color_settling = raw_t < 1.0 && entry.start_rgb != entry.target_rgb;
        for i in 0..3 {
            entry.current_rgb[i] =
                entry.start_rgb[i] + (entry.target_rgb[i] - entry.start_rgb[i]) * eased;
        }

        let elapsed_state = now
            .saturating_duration_since(entry.state_entered_at)
            .as_secs_f32();

        let mut border_color = Color32::from_rgb(
            entry.current_rgb[0].round().clamp(0.0, 255.0) as u8,
            entry.current_rgb[1].round().clamp(0.0, 255.0) as u8,
            entry.current_rgb[2].round().clamp(0.0, 255.0) as u8,
        );

        let mut outer_glow_color = None;
        let mut outer_glow_width = 0.0;
        let mut continuous_anim = false;
        let mut border_width = if is_focused { 1.6 } else { 1.0 };

        match status {
            AgentStatus::Working | AgentStatus::Thinking => {
                // Gentle breathing pulse while thinking or actively working.
                let speed = if status == AgentStatus::Thinking {
                    2.4
                } else {
                    3.2
                };
                let wave = 0.5 + 0.5 * (elapsed_state * speed).sin();
                let glow_alpha = (28.0 + wave * 65.0) as u8;
                outer_glow_color = Some(Color32::from_rgba_unmultiplied(
                    border_color.r(),
                    border_color.g(),
                    border_color.b(),
                    glow_alpha,
                ));
                outer_glow_width = 3.5 + wave * 1.5;
                border_width = if is_focused { 2.0 } else { 1.6 };
                continuous_anim = true;
            }
            AgentStatus::RunningTool => {
                // Crisper faster pulse for active tool invocation.
                let wave = 0.5 + 0.5 * (elapsed_state * 5.2).sin();
                let glow_alpha = (42.0 + wave * 75.0) as u8;
                outer_glow_color = Some(Color32::from_rgba_unmultiplied(
                    border_color.r(),
                    border_color.g(),
                    border_color.b(),
                    glow_alpha,
                ));
                outer_glow_width = 4.0 + wave * 1.8;
                border_width = if is_focused { 2.2 } else { 1.8 };
                continuous_anim = true;
            }
            AgentStatus::WaitingForInput => {
                // Warm attention amber glow when awaiting user reply.
                let wave = 0.5 + 0.5 * (elapsed_state * 3.8).sin();
                let glow_alpha = (55.0 + wave * 80.0) as u8;
                outer_glow_color = Some(Color32::from_rgba_unmultiplied(
                    border_color.r(),
                    border_color.g(),
                    border_color.b(),
                    glow_alpha,
                ));
                outer_glow_width = 4.5 + wave * 2.0;
                border_width = 2.6;
                continuous_anim = true;
            }
            AgentStatus::Done => {
                border_width = 2.2;
                if elapsed_state < DONE_FLASH_SECS {
                    let decay = 1.0 - (elapsed_state / DONE_FLASH_SECS);
                    let flash_alpha = (decay * 135.0) as u8;
                    border_color = lerp_color(border_color, Color32::WHITE, decay * 0.22);
                    outer_glow_color = Some(Color32::from_rgba_unmultiplied(
                        border_color.r(),
                        border_color.g(),
                        border_color.b(),
                        flash_alpha,
                    ));
                    outer_glow_width = 4.0 + decay * 4.0;
                    continuous_anim = true;
                }
            }
            AgentStatus::Failed => {
                border_width = 2.6;
                if elapsed_state < FAILED_PULSE_SECS {
                    let decay = 1.0 - (elapsed_state / FAILED_PULSE_SECS);
                    let flash_alpha = (decay * 130.0) as u8;
                    outer_glow_color = Some(Color32::from_rgba_unmultiplied(
                        border_color.r(),
                        border_color.g(),
                        border_color.b(),
                        flash_alpha,
                    ));
                    outer_glow_width = 4.5 + decay * 2.5;
                    continuous_anim = true;
                }
            }
            AgentStatus::Idle => {
                if team_color.is_some() && !is_focused {
                    border_width = 1.4;
                }
            }
        }

        // Preserve unmistakable focus hairline when pane has keyboard focus in a non-default state.
        let focus_inner_color = if is_focused && status != AgentStatus::Idle {
            Some(Color32::from_rgb(
                theme.active_border.0,
                theme.active_border.1,
                theme.active_border.2,
            ))
        } else {
            None
        };

        PaneBorderVisual {
            border_color,
            border_width,
            outer_glow_color,
            outer_glow_width,
            focus_inner_color,
            animating: color_settling || continuous_anim,
        }
    }
}

fn target_rgb_for_state(
    status: AgentStatus,
    is_focused: bool,
    team_color: Option<(u8, u8, u8)>,
    theme: &ResolvedTheme,
) -> (u8, u8, u8) {
    match status {
        AgentStatus::WaitingForInput => theme.ring_waiting,
        AgentStatus::Done => theme.ring_done,
        AgentStatus::Failed => theme.ring_failed,
        AgentStatus::Working | AgentStatus::Thinking => working_glow_rgb(theme),
        AgentStatus::RunningTool => tool_glow_rgb(theme),
        AgentStatus::Idle => {
            if is_focused {
                theme.active_border
            } else if let Some(tc) = team_color {
                tc
            } else {
                theme.inactive_border
            }
        }
    }
}

fn working_glow_rgb(theme: &ResolvedTheme) -> (u8, u8, u8) {
    let (ar, ag, ab) = theme.active_border;
    let (dr, dg, db) = theme.ring_done;
    (
        ((ar as u16 * 3 + dr as u16) / 4) as u8,
        ((ag as u16 * 3 + dg as u16) / 4) as u8,
        ((ab as u16 * 3 + db as u16) / 4) as u8,
    )
}

fn tool_glow_rgb(theme: &ResolvedTheme) -> (u8, u8, u8) {
    let (ar, ag, ab) = theme.active_border;
    let (wr, wg, wb) = theme.ring_waiting;
    (
        ((ar as u16 * 3 + wr as u16) / 4) as u8,
        ((ag as u16 * 3 + wg as u16) / 4) as u8,
        ((ab as u16 * 3 + wb as u16) / 4) as u8,
    )
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let k = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * k).round() as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * k).round() as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * k).round() as u8,
    )
}

/// Draw a compact, clean-room procedural vector Agent Mascot reflecting session state (`REQ-085`).
pub fn render_agent_mascot(
    painter: &egui::Painter,
    rect: Rect,
    status: AgentStatus,
    time_secs: f64,
    theme: &ResolvedTheme,
) {
    let w = rect.width().min(rect.height()).max(12.0);
    let c = rect.center();

    let accent = match status {
        AgentStatus::Idle => Color32::from_rgb(
            theme.inactive_border.0.saturating_add(45),
            theme.inactive_border.1.saturating_add(45),
            theme.inactive_border.2.saturating_add(55),
        ),
        AgentStatus::Working | AgentStatus::Thinking => Color32::from_rgb(
            theme.active_border.0,
            theme.active_border.1,
            theme.active_border.2,
        ),
        AgentStatus::RunningTool => {
            let rgb = tool_glow_rgb(theme);
            Color32::from_rgb(rgb.0, rgb.1, rgb.2)
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
    };

    let bob_y = match status {
        AgentStatus::Thinking => (time_secs * 3.8).sin() as f32 * (w * 0.05),
        AgentStatus::Working | AgentStatus::RunningTool => {
            (time_secs * 7.2).sin() as f32 * (w * 0.06)
        }
        AgentStatus::Done => -((time_secs * 7.5).sin().abs() as f32) * (w * 0.08),
        AgentStatus::Failed => (time_secs * 18.0).sin() as f32 * (w * 0.04),
        AgentStatus::WaitingForInput => (time_secs * 3.5).sin() as f32 * (w * 0.04),
        AgentStatus::Idle => 0.0,
    };

    let center = Pos2::new(c.x, c.y + bob_y);
    let body_rect = Rect::from_center_size(
        Pos2::new(center.x, center.y + w * 0.06),
        Vec2::new(w * 0.78, w * 0.62),
    );

    // Antenna stem + tip
    let tip_offset_x = if status == AgentStatus::Failed {
        w * 0.10
    } else {
        0.0
    };
    let stem_base = Pos2::new(center.x, body_rect.min.y);
    let stem_tip = Pos2::new(center.x + tip_offset_x, body_rect.min.y - w * 0.16);
    painter.line_segment([stem_base, stem_tip], Stroke::new(1.3, accent));
    painter.circle_filled(stem_tip, (w * 0.08).max(1.5), accent);

    // Bot head capsule & dark visor
    let head_fill = if theme.is_light_background() {
        Color32::from_rgb(235, 240, 248)
    } else {
        Color32::from_rgb(28, 32, 46)
    };
    painter.rect_filled(body_rect, w * 0.18, head_fill);
    painter.rect_stroke(
        body_rect,
        w * 0.18,
        Stroke::new(1.3, accent),
        egui::StrokeKind::Inside,
    );

    let eye_y = body_rect.center().y;
    let eye_dx = w * 0.16;
    let left_eye = Pos2::new(center.x - eye_dx, eye_y);
    let right_eye = Pos2::new(center.x + eye_dx, eye_y);

    match status {
        AgentStatus::Idle => {
            // Sleeping closed eyes
            let ew = w * 0.08;
            painter.line_segment(
                [
                    Pos2::new(left_eye.x - ew, eye_y),
                    Pos2::new(left_eye.x + ew, eye_y),
                ],
                Stroke::new(1.3, accent),
            );
            painter.line_segment(
                [
                    Pos2::new(right_eye.x - ew, eye_y),
                    Pos2::new(right_eye.x + ew, eye_y),
                ],
                Stroke::new(1.3, accent),
            );
        }
        AgentStatus::Thinking => {
            let er = (w * 0.075).max(1.4);
            let blink = 0.55 + 0.45 * (time_secs * 3.5).sin() as f32;
            let glow = Color32::from_rgba_unmultiplied(
                accent.r(),
                accent.g(),
                accent.b(),
                (blink * 255.0) as u8,
            );
            painter.circle_filled(left_eye, er, glow);
            painter.circle_filled(right_eye, er, glow);
        }
        AgentStatus::Working | AgentStatus::RunningTool => {
            // Active glowing visor dots + alternating typing/tool paws
            let er = (w * 0.075).max(1.4);
            painter.circle_filled(left_eye, er, accent);
            painter.circle_filled(right_eye, er, accent);

            let freq = if status == AgentStatus::RunningTool {
                12.0
            } else {
                9.0
            };
            let tap_phase = (time_secs * freq).sin() as f32;
            let paw_y_l = body_rect.max.y + w * 0.05 + tap_phase * (w * 0.05);
            let paw_y_r = body_rect.max.y + w * 0.05 - tap_phase * (w * 0.05);
            painter.circle_filled(
                Pos2::new(center.x - w * 0.18, paw_y_l),
                (w * 0.07).max(1.4),
                accent,
            );
            painter.circle_filled(
                Pos2::new(center.x + w * 0.18, paw_y_r),
                (w * 0.07).max(1.4),
                accent,
            );
        }
        AgentStatus::WaitingForInput => {
            // Wide alert eyes
            let er = (w * 0.095).max(1.8);
            painter.circle_stroke(left_eye, er, Stroke::new(1.3, accent));
            painter.circle_stroke(right_eye, er, Stroke::new(1.3, accent));
        }
        AgentStatus::Done => {
            // Happy chevron eyes ^ ^
            let ew = w * 0.08;
            let eh = w * 0.06;
            for eye in [left_eye, right_eye] {
                painter.line_segment(
                    [
                        Pos2::new(eye.x - ew, eye.y + eh * 0.5),
                        Pos2::new(eye.x, eye.y - eh),
                    ],
                    Stroke::new(1.4, accent),
                );
                painter.line_segment(
                    [
                        Pos2::new(eye.x, eye.y - eh),
                        Pos2::new(eye.x + ew, eye.y + eh * 0.5),
                    ],
                    Stroke::new(1.4, accent),
                );
            }
        }
        AgentStatus::Failed => {
            // Panic X X eyes
            let ew = w * 0.075;
            for eye in [left_eye, right_eye] {
                painter.line_segment(
                    [
                        Pos2::new(eye.x - ew, eye.y - ew),
                        Pos2::new(eye.x + ew, eye.y + ew),
                    ],
                    Stroke::new(1.4, accent),
                );
                painter.line_segment(
                    [
                        Pos2::new(eye.x - ew, eye.y + ew),
                        Pos2::new(eye.x + ew, eye.y - ew),
                    ],
                    Stroke::new(1.4, accent),
                );
            }
        }
    }
}

/// Frame result of pane layout easing & debounced ConPTY resize gating (`REQ-089`).
#[derive(Debug, Clone, Copy)]
pub struct AnimatedPaneRect {
    pub visual_rect: Rect,
    pub allow_backend_resize: bool,
    pub animating: bool,
}

#[derive(Debug, Clone)]
struct PaneRectState {
    start_rect: Rect,
    current_rect: Rect,
    target_rect: Rect,
    anim_started_at: Instant,
    target_changed_at: Instant,
    is_initial: bool,
}

#[derive(Debug, Clone)]
struct ClosingPaneGhost {
    rect: Rect,
    started_at: Instant,
    fill_color: Color32,
    border_color: Color32,
}

/// Smooth ~140 ms open/close/resize visual interpolator paired with ConPTY resize debouncing (`REQ-089`).
#[derive(Debug, Default)]
pub struct PaneRectAnimator {
    rects: HashMap<PaneId, PaneRectState>,
    closing_ghosts: Vec<ClosingPaneGhost>,
}

impl PaneRectAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_close(
        &mut self,
        pane_id: PaneId,
        fill_color: Color32,
        border_color: Color32,
        now: Instant,
    ) {
        if let Some(st) = self.rects.remove(&pane_id) {
            self.closing_ghosts.push(ClosingPaneGhost {
                rect: st.current_rect,
                started_at: now,
                fill_color,
                border_color,
            });
        }
    }

    /// Interpolate a pane's visual container while debouncing ConPTY/WebView2 resizes until settled.
    pub fn step(
        &mut self,
        pane_id: PaneId,
        target: Rect,
        dragging_split: bool,
        now: Instant,
    ) -> AnimatedPaneRect {
        if dragging_split {
            let state = self.rects.entry(pane_id).or_insert(PaneRectState {
                start_rect: target,
                current_rect: target,
                target_rect: target,
                anim_started_at: now,
                target_changed_at: now,
                is_initial: true,
            });
            state.start_rect = target;
            state.current_rect = target;
            state.target_rect = target;
            state.target_changed_at = now;
            state.is_initial = false;
            return AnimatedPaneRect {
                visual_rect: target,
                allow_backend_resize: true,
                animating: false,
            };
        }

        let state = self.rects.entry(pane_id).or_insert_with(|| {
            // Subtle crisp expand-in on newly opened pane
            let inset_start = Rect::from_center_size(
                target.center(),
                Vec2::new(
                    (target.width() * 0.88).max(32.0),
                    (target.height() * 0.88).max(32.0),
                ),
            );
            PaneRectState {
                start_rect: inset_start,
                current_rect: inset_start,
                target_rect: target,
                anim_started_at: now,
                target_changed_at: now,
                is_initial: true,
            }
        });

        if !rects_close(state.target_rect, target) {
            state.start_rect = state.current_rect;
            state.target_rect = target;
            state.anim_started_at = now;
            state.target_changed_at = now;
            state.is_initial = false;
        }

        let elapsed = now
            .saturating_duration_since(state.anim_started_at)
            .as_secs_f32();
        let raw_t = (elapsed / PANE_RECT_EASE_SECS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - raw_t).powi(3);
        let visual_rect = lerp_rect(state.start_rect, state.target_rect, eased);
        state.current_rect = visual_rect;

        let settled = raw_t >= 1.0;
        let since_change = now
            .saturating_duration_since(state.target_changed_at)
            .as_secs_f32();
        let allow_backend_resize =
            state.is_initial || settled || since_change >= CONPTY_RESIZE_DEBOUNCE_SECS;

        AnimatedPaneRect {
            visual_rect,
            allow_backend_resize,
            animating: !settled,
        }
    }

    /// Render fading/shrinking close ghosts for recently closed panes; returns `true` if still animating.
    pub fn render_closing_ghosts(&mut self, painter: &egui::Painter, now: Instant) -> bool {
        self.closing_ghosts.retain(|g| {
            let elapsed = now.saturating_duration_since(g.started_at).as_secs_f32();
            if elapsed >= PANE_RECT_EASE_SECS {
                return false;
            }
            let t = (elapsed / PANE_RECT_EASE_SECS).clamp(0.0, 1.0);
            let scale = 1.0 - 0.12 * t;
            let r = Rect::from_center_size(
                g.rect.center(),
                Vec2::new(g.rect.width() * scale, g.rect.height() * scale),
            );
            let alpha = ((1.0 - t) * 185.0) as u8;
            painter.rect_filled(
                r,
                4.0,
                Color32::from_rgba_unmultiplied(
                    g.fill_color.r(),
                    g.fill_color.g(),
                    g.fill_color.b(),
                    alpha,
                ),
            );
            painter.rect_stroke(
                r,
                4.0,
                Stroke::new(
                    1.5,
                    Color32::from_rgba_unmultiplied(
                        g.border_color.r(),
                        g.border_color.g(),
                        g.border_color.b(),
                        alpha,
                    ),
                ),
                egui::StrokeKind::Inside,
            );
            true
        });
        !self.closing_ghosts.is_empty()
    }
}

fn rects_close(a: Rect, b: Rect) -> bool {
    (a.min.x - b.min.x).abs() < 0.5
        && (a.min.y - b.min.y).abs() < 0.5
        && (a.max.x - b.max.x).abs() < 0.5
        && (a.max.y - b.max.y).abs() < 0.5
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    let f = |x: f32, y: f32| x + (y - x) * t;
    Rect::from_min_max(
        Pos2::new(f(a.min.x, b.min.x), f(a.min.y, b.min.y)),
        Pos2::new(f(a.max.x, b.max.x), f(a.max.y, b.max.y)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use agemax_config::theme::{ThemeConfig, ThemePreset};

    #[test]
    fn test_animator_eases_and_stops_on_idle() {
        let mut anim = PaneVisualAnimator::new();
        let pid = PaneId::new();
        let theme = ThemeConfig::from_preset(ThemePreset::MidnightDark).resolved_rgb();
        let t0 = Instant::now();

        let working = anim.compute(pid, AgentStatus::Working, true, None, &theme, t0);
        assert!(working.animating);
        assert!(working.outer_glow_color.is_some());
        assert!(working.focus_inner_color.is_some());

        // Advance past ease duration in Idle -> settles calmly with animating=false
        let mut t = t0;
        let mut settled = working;
        for _ in 0..8 {
            t += std::time::Duration::from_millis(50);
            settled = anim.compute(pid, AgentStatus::Idle, false, None, &theme, t);
        }
        assert!(!settled.animating);
        assert!(settled.outer_glow_color.is_none());
    }

    #[test]
    fn test_done_flash_settles_after_duration() {
        let mut anim = PaneVisualAnimator::new();
        let pid = PaneId::new();
        let theme = ThemeConfig::from_preset(ThemePreset::Dracula).resolved_rgb();
        let t0 = Instant::now();

        let initial_done = anim.compute(pid, AgentStatus::Done, false, None, &theme, t0);
        assert!(initial_done.animating);
        assert!(initial_done.outer_glow_color.is_some());

        let later = t0 + std::time::Duration::from_secs(2);
        let settled_done = anim.compute(pid, AgentStatus::Done, false, None, &theme, later);
        assert!(!settled_done.animating);
        assert!(settled_done.outer_glow_color.is_none());
    }

    #[test]
    fn test_pane_rect_animator_debounces_resize_and_settles() {
        let mut rect_anim = PaneRectAnimator::new();
        let pid = PaneId::new();
        let r1 = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let r2 = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 600.0));
        let t0 = Instant::now();

        // First appearance allows initial PTY sizing cleanly
        let a0 = rect_anim.step(pid, r1, false, t0);
        assert!(a0.animating);
        assert!(a0.allow_backend_resize);

        // Settle first animation
        let t1 = t0 + std::time::Duration::from_millis(200);
        let a1 = rect_anim.step(pid, r1, false, t1);
        assert!(!a1.animating);

        // Layout splits to r2 -> debounces intermediate PTY resize on frame 1 (30ms)
        let a2 = rect_anim.step(pid, r2, false, t1 + std::time::Duration::from_millis(30));
        assert!(a2.animating);
        assert!(!a2.allow_backend_resize);

        // Once eased past 140ms, resize commits and animation sleeps cleanly
        let a3 = rect_anim.step(pid, r2, false, t1 + std::time::Duration::from_millis(200));
        assert!(!a3.animating);
        assert!(a3.allow_backend_resize);
        assert_eq!(a3.visual_rect, r2);
    }
}
