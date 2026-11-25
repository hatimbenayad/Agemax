//! Live per-pane activity telemetry, zero-allocation sparkline renderer,
//! and throttled background git diff statistics worker (`REQ-013`, `REQ-087`).

use agemax_core::layout::WorkspaceId;
use agemax_core::status::AgentStatus;
use agemax_term::metadata::{detect_git_diff_stats, GitDiffStat};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, Color32, Pos2, Rect};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub const SPARKLINE_BUCKETS: usize = 16;

/// Zero-allocation rolling activity & timing telemetry per pane (`REQ-087`).
#[derive(Debug, Clone)]
pub struct PaneTelemetry {
    pub sparkline: [u32; SPARKLINE_BUCKETS],
    last_bucket_tick: Instant,
    pub last_output_line: Option<String>,
    pub work_started_at: Option<Instant>,
    pub last_pty_output_at: Option<Instant>,
    pub heuristic_active: bool,
}

impl Default for PaneTelemetry {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

impl PaneTelemetry {
    pub fn new(now: Instant) -> Self {
        Self {
            sparkline: [0; SPARKLINE_BUCKETS],
            last_bucket_tick: now,
            last_output_line: None,
            work_started_at: None,
            last_pty_output_at: None,
            heuristic_active: false,
        }
    }

    /// Shift 1-second buckets forward as wall time advances without allocating memory.
    pub fn advance_time(&mut self, now: Instant) {
        let elapsed_secs = now
            .saturating_duration_since(self.last_bucket_tick)
            .as_secs();
        if elapsed_secs == 0 {
            return;
        }
        let steps = (elapsed_secs as usize).min(SPARKLINE_BUCKETS);
        if steps == SPARKLINE_BUCKETS {
            self.sparkline = [0; SPARKLINE_BUCKETS];
        } else {
            self.sparkline.copy_within(steps..SPARKLINE_BUCKETS, 0);
            for slot in &mut self.sparkline[SPARKLINE_BUCKETS - steps..] {
                *slot = 0;
            }
        }
        self.last_bucket_tick = now;
    }

    pub fn record_bytes(&mut self, bytes_len: usize, now: Instant) {
        self.advance_time(now);
        let slot = &mut self.sparkline[SPARKLINE_BUCKETS - 1];
        *slot = slot.saturating_add(bytes_len as u32);
        self.last_pty_output_at = Some(now);
    }

    pub fn sync_status(&mut self, status: AgentStatus, now: Instant) {
        if status.is_working() {
            if self.work_started_at.is_none() {
                self.work_started_at = Some(now);
            }
        } else {
            self.work_started_at = None;
        }
    }

    pub fn working_elapsed_secs(&self, now: Instant) -> Option<u64> {
        self.work_started_at
            .map(|start| now.saturating_duration_since(start).as_secs())
    }
}

/// Infer coarse `Thinking` vs `RunningTool` hint from PTY byte stream when explicit OSC 7501 isn't emitted.
pub fn infer_pty_tool_hint(chunk: &[u8]) -> Option<AgentStatus> {
    let text = String::from_utf8_lossy(chunk);
    let lower = text.to_ascii_lowercase();
    if lower.contains("running ")
        || lower.contains("executing ")
        || lower.contains("compiling ")
        || lower.contains("testing ")
        || lower.contains("tool_call")
    {
        Some(AgentStatus::RunningTool)
    } else if chunk.len() >= 24 {
        Some(AgentStatus::Thinking)
    } else {
        None
    }
}

pub fn format_elapsed_compact(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Draw a tiny vector bar sparkline with zero heap allocations (`REQ-087`).
pub fn render_sparkline(
    painter: &egui::Painter,
    rect: Rect,
    samples: &[u32; SPARKLINE_BUCKETS],
    accent: Color32,
    base: Color32,
) {
    let max_val = samples.iter().copied().max().unwrap_or(0).max(64) as f32;
    let bar_w = (rect.width() / SPARKLINE_BUCKETS as f32).max(1.5);
    let gap = 0.8_f32.min(bar_w * 0.25);

    for (i, &val) in samples.iter().enumerate() {
        let ratio = (val as f32 / max_val).sqrt().clamp(0.0, 1.0);
        let h = (rect.height() * ratio).max(if val > 0 { 2.0 } else { 1.0 });
        let x0 = rect.min.x + i as f32 * bar_w;
        let x1 = (x0 + bar_w - gap).max(x0 + 1.0);
        let bar_rect = Rect::from_min_max(Pos2::new(x0, rect.max.y - h), Pos2::new(x1, rect.max.y));
        let col = if val > 0 {
            Color32::from_rgba_unmultiplied(
                accent.r(),
                accent.g(),
                accent.b(),
                (110.0 + ratio * 145.0) as u8,
            )
        } else {
            base
        };
        painter.rect_filled(bar_rect, 0.8, col);
    }
}

/// Throttled background worker computing git diff churn stats (`+N -M`) without blocking UI (`REQ-013`).
pub struct GitStatsPoller {
    req_tx: Sender<Vec<(WorkspaceId, PathBuf)>>,
    res_rx: Receiver<HashMap<WorkspaceId, GitDiffStat>>,
    pub cached: HashMap<WorkspaceId, GitDiffStat>,
    last_request: Instant,
}

impl GitStatsPoller {
    pub fn spawn(ctx: Option<egui::Context>) -> Self {
        let (req_tx, req_rx) = crossbeam_channel::bounded::<Vec<(WorkspaceId, PathBuf)>>(2);
        let (res_tx, res_rx) = crossbeam_channel::bounded::<HashMap<WorkspaceId, GitDiffStat>>(2);

        std::thread::Builder::new()
            .name("agemax-git-stats".to_string())
            .spawn(move || {
                while let Ok(targets) = req_rx.recv() {
                    let mut map = HashMap::new();
                    for (ws_id, cwd) in targets {
                        if let Some(stat) = detect_git_diff_stats(&cwd) {
                            map.insert(ws_id, stat);
                        }
                    }
                    if res_tx.try_send(map).is_ok() {
                        if let Some(ref c) = ctx {
                            c.request_repaint();
                        }
                    }
                }
            })
            .ok();

        Self {
            req_tx,
            res_rx,
            cached: HashMap::new(),
            last_request: Instant::now() - std::time::Duration::from_secs(10),
        }
    }

    pub fn poll_and_maybe_refresh(
        &mut self,
        targets: impl FnOnce() -> Vec<(WorkspaceId, PathBuf)>,
        now: Instant,
    ) {
        while let Ok(map) = self.res_rx.try_recv() {
            self.cached = map;
        }
        if now.saturating_duration_since(self.last_request).as_secs() >= 4 {
            let batch = targets();
            if !batch.is_empty() {
                let _ = self.req_tx.try_send(batch);
            }
            self.last_request = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_telemetry_sparkline_shift_and_elapsed() {
        let t0 = Instant::now();
        let mut tel = PaneTelemetry::new(t0);
        tel.record_bytes(256, t0);
        assert_eq!(tel.sparkline[SPARKLINE_BUCKETS - 1], 256);

        let t3 = t0 + std::time::Duration::from_secs(3);
        tel.record_bytes(128, t3);
        assert_eq!(tel.sparkline[SPARKLINE_BUCKETS - 4], 256);
        assert_eq!(tel.sparkline[SPARKLINE_BUCKETS - 1], 128);

        tel.sync_status(AgentStatus::Thinking, t3);
        assert_eq!(
            tel.working_elapsed_secs(t3 + std::time::Duration::from_secs(65)),
            Some(65)
        );
        assert_eq!(format_elapsed_compact(65), "1m 05s");
    }
}
