//! Live per-pane activity telemetry, zero-allocation sparkline renderer,
//! and throttled background git diff statistics worker (`REQ-013`, `REQ-087`).

use agemax_core::layout::WorkspaceId;
use agemax_core::status::AgentStatus;
use agemax_term::metadata::{detect_git_branch, detect_git_diff_stats, GitDiffStat};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use eframe::egui::{self, Color32, Pos2, Rect};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

/// Draw a smooth zero-allocation vector polyline sparkline (`REQ-087`).
pub fn render_sparkline(
    painter: &egui::Painter,
    rect: Rect,
    samples: &[u32; SPARKLINE_BUCKETS],
    accent: Color32,
    base: Color32,
) {
    let max_val = samples.iter().copied().max().unwrap_or(0);
    let norm_max = max_val.max(64) as f32;
    let has_activity = max_val > 0;
    let line_col = if has_activity { accent } else { base };

    let mut pts = [Pos2::ZERO; SPARKLINE_BUCKETS];
    let denom = (SPARKLINE_BUCKETS.saturating_sub(1)).max(1) as f32;
    let usable_h = (rect.height() - 3.0).max(2.0);
    let baseline_y = rect.max.y - 1.5;

    for (i, &val) in samples.iter().enumerate() {
        let ratio = (val as f32 / norm_max).sqrt().clamp(0.0, 1.0);
        let x = rect.min.x + (i as f32 / denom) * rect.width();
        let y = baseline_y - ratio * usable_h;
        pts[i] = Pos2::new(x, y);
    }

    for pair in pts.windows(2) {
        painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, line_col));
    }
}

struct GitBatchSample {
    stats: HashMap<WorkspaceId, GitDiffStat>,
    branches: HashMap<WorkspaceId, Option<String>>,
}

/// Throttled background worker computing git diff churn stats (`+N -M`) and live branch names without blocking UI (`REQ-013`).
pub struct GitStatsPoller {
    req_tx: Sender<Vec<(WorkspaceId, PathBuf)>>,
    res_rx: Receiver<GitBatchSample>,
    pub cached: HashMap<WorkspaceId, GitDiffStat>,
    pub branches: HashMap<WorkspaceId, Option<String>>,
    last_request: Instant,
}

impl GitStatsPoller {
    pub fn spawn(ctx: Option<egui::Context>) -> Self {
        let (req_tx, req_rx) = crossbeam_channel::bounded::<Vec<(WorkspaceId, PathBuf)>>(2);
        let (res_tx, res_rx) = crossbeam_channel::bounded::<GitBatchSample>(2);

        std::thread::Builder::new()
            .name("agemax-git-stats".to_string())
            .spawn(move || {
                while let Ok(targets) = req_rx.recv() {
                    let mut stats = HashMap::new();
                    let mut branches = HashMap::new();
                    for (ws_id, cwd) in targets {
                        if let Some(stat) = detect_git_diff_stats(&cwd) {
                            stats.insert(ws_id, stat);
                        }
                        branches.insert(ws_id, detect_git_branch(&cwd));
                    }
                    if res_tx.try_send(GitBatchSample { stats, branches }).is_ok() {
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
            branches: HashMap::new(),
            last_request: Instant::now() - Duration::from_secs(10),
        }
    }

    pub fn poll_and_maybe_refresh(
        &mut self,
        targets: impl FnOnce() -> Vec<(WorkspaceId, PathBuf)>,
        now: Instant,
    ) -> bool {
        let mut updated = false;
        while let Ok(sample) = self.res_rx.try_recv() {
            self.cached = sample.stats;
            self.branches = sample.branches;
            updated = true;
        }
        if now.saturating_duration_since(self.last_request).as_secs() >= 4 {
            let batch = targets();
            if !batch.is_empty() {
                let _ = self.req_tx.try_send(batch);
            }
            self.last_request = now;
        }
        updated
    }
}

/// System-wide CPU & RAM load percentages sampled on Windows without third-party crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemStats {
    pub cpu_percent: u8,
    pub ram_percent: u8,
}

/// Pure CPU utilization calculator from two cumulative Win32 `GetSystemTimes` snapshots.
/// Note that on Windows `kernel` time includes `idle` time.
pub fn compute_cpu_percent(
    prev_idle: u64,
    prev_kernel: u64,
    prev_user: u64,
    cur_idle: u64,
    cur_kernel: u64,
    cur_user: u64,
) -> Option<u8> {
    let kernel_delta = cur_kernel.checked_sub(prev_kernel)?;
    let user_delta = cur_user.checked_sub(prev_user)?;
    let idle_delta = cur_idle.checked_sub(prev_idle)?;
    let total = kernel_delta.saturating_add(user_delta);
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle_delta);
    let pct = ((busy as f64 / total as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0);
    Some(pct as u8)
}

#[cfg(windows)]
fn filetime_to_u64(ft: windows::Win32::Foundation::FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

#[cfg(windows)]
fn sample_system_times() -> Option<(u64, u64, u64)> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;

    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: valid mutable out-pointers for Win32 GetSystemTimes.
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    Some((
        filetime_to_u64(idle),
        filetime_to_u64(kernel),
        filetime_to_u64(user),
    ))
}

#[cfg(windows)]
fn sample_ram_percent() -> Option<u8> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut mem = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `dwLength` initialized cleanly before calling GlobalMemoryStatusEx.
    unsafe { GlobalMemoryStatusEx(&mut mem) }.ok()?;
    Some(mem.dwMemoryLoad.min(100) as u8)
}

#[cfg(not(windows))]
fn sample_system_times() -> Option<(u64, u64, u64)> {
    None
}

#[cfg(not(windows))]
fn sample_ram_percent() -> Option<u8> {
    None
}

/// Throttled background worker sampling CPU and RAM load every 2 seconds.
pub struct SystemStatsPoller {
    res_rx: Receiver<SystemStats>,
    _stop_tx: Sender<()>,
}

impl SystemStatsPoller {
    pub fn spawn(ctx: Option<egui::Context>) -> Self {
        let (res_tx, res_rx) = crossbeam_channel::bounded::<SystemStats>(2);
        let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);

        std::thread::Builder::new()
            .name("agemax-sys-stats".to_string())
            .spawn(move || {
                let mut prev_times = sample_system_times();
                let mut last_sent: Option<SystemStats> = None;

                // Brief initial delay so CPU% gets a real two-point delta quickly after launch.
                match stop_rx.recv_timeout(Duration::from_millis(250)) {
                    Ok(_) | Err(RecvTimeoutError::Disconnected) => return,
                    Err(RecvTimeoutError::Timeout) => {}
                }

                loop {
                    let cur_times = sample_system_times();
                    let cpu_percent = match (prev_times, cur_times) {
                        (Some((pi, pk, pu)), Some((ci, ck, cu))) => {
                            compute_cpu_percent(pi, pk, pu, ci, ck, cu).unwrap_or(0)
                        }
                        _ => 0,
                    };
                    if cur_times.is_some() {
                        prev_times = cur_times;
                    }
                    let ram_percent = sample_ram_percent().unwrap_or(0);
                    let stats = SystemStats {
                        cpu_percent,
                        ram_percent,
                    };

                    if last_sent != Some(stats) && res_tx.try_send(stats).is_ok() {
                        last_sent = Some(stats);
                        if let Some(ref c) = ctx {
                            c.request_repaint();
                        }
                    }

                    match stop_rx.recv_timeout(Duration::from_secs(2)) {
                        Ok(_) | Err(RecvTimeoutError::Disconnected) => break,
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                }
            })
            .ok();

        Self {
            res_rx,
            _stop_tx: stop_tx,
        }
    }

    pub fn poll(&mut self) -> Option<SystemStats> {
        let mut latest = None;
        while let Ok(stats) = self.res_rx.try_recv() {
            latest = Some(stats);
        }
        latest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cpu_percent_math() {
        // kernel_delta = 100 (includes 25 idle), user_delta = 0 -> busy = 75%
        assert_eq!(compute_cpu_percent(10, 50, 20, 35, 150, 20), Some(75));
        // Zero total delta returns None cleanly
        assert_eq!(compute_cpu_percent(10, 50, 20, 10, 50, 20), None);
        // Counter decrease returns None safely without underflow
        assert_eq!(compute_cpu_percent(100, 200, 50, 90, 190, 50), None);
    }

    #[test]
    fn test_system_stats_poller_live_windows() {
        let times = sample_system_times();
        let ram = sample_ram_percent();
        #[cfg(windows)]
        {
            assert!(times.is_some(), "sample_system_times failed");
            assert!(ram.is_some(), "sample_ram_percent failed");
        }
        let _ = (times, ram);
    }

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
