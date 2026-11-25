use agemax_core::notification::NotificationItem;
use agemax_core::status::AgentStatus;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(windows)]
use windows::core::{BOOL, HSTRING, PCWSTR};
#[cfg(windows)]
use windows::Data::Xml::Dom::XmlDocument;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM};
#[cfg(windows)]
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FlashWindowEx, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, SetForegroundWindow, ShowWindow, FLASHWINFO, FLASHW_TIMERNOFG, FLASHW_TRAY,
    SW_RESTORE,
};
#[cfg(windows)]
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

const CHIME_COOLDOWN: Duration = Duration::from_millis(2500);

pub trait NotificationService: Send + Sync {
    fn show_notification(&self, item: &NotificationItem);
    fn flash_window(&self);
    fn activate_window(&self);
    fn play_chime(&self);
}

pub struct PlatformNotificationService {
    last_chime: Mutex<Option<Instant>>,
}

impl PlatformNotificationService {
    pub fn new() -> Self {
        Self {
            last_chime: Mutex::new(None),
        }
    }

    fn should_chime_now(&self, now: Instant) -> bool {
        let Ok(mut guard) = self.last_chime.lock() else {
            return false;
        };
        if let Some(prev) = *guard {
            if now.saturating_duration_since(prev) < CHIME_COOLDOWN {
                return false;
            }
        }
        *guard = Some(now);
        true
    }
}

impl Default for PlatformNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationService for PlatformNotificationService {
    fn show_notification(&self, item: &NotificationItem) {
        #[cfg(windows)]
        {
            let title = match item.status {
                AgentStatus::WaitingForInput => "Age Max: Agent Needs Input",
                AgentStatus::Done => "Age Max: Agent Finished",
                AgentStatus::Failed => "Age Max: Agent Failed",
                _ => "Age Max Notification",
            };
            let body = &item.message;

            if let Err(e) = show_windows_toast(title, body) {
                tracing::debug!("Native Windows toast delivery failed: {e:?}, using fallback");
            }
        }

        #[cfg(not(windows))]
        {
            let _ = item;
        }
    }

    fn flash_window(&self) {
        #[cfg(windows)]
        unsafe {
            let hwnd = find_current_process_hwnd().unwrap_or_else(|| GetForegroundWindow());
            if hwnd != HWND(std::ptr::null_mut()) {
                let info = FLASHWINFO {
                    cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                    hwnd,
                    dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
                    uCount: 3,
                    dwTimeout: 0,
                };
                let _ = FlashWindowEx(&info);
            }
        }
    }

    fn activate_window(&self) {
        #[cfg(windows)]
        unsafe {
            if let Some(hwnd) = find_current_process_hwnd() {
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    fn play_chime(&self) {
        if !self.should_chime_now(Instant::now()) {
            return;
        }
        #[cfg(windows)]
        {
            let wav_words = chime_wav_words();
            unsafe {
                let _ = PlaySoundW(
                    PCWSTR(wav_words.as_ptr()),
                    None,
                    SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
                );
            }
        }
    }
}

/// Synthesize a subtle clean-room dual-tone sine chime (22.05 kHz, 16-bit mono PCM WAV)
/// packed in aligned `u16` little-endian words so `PlaySoundW(SND_MEMORY | SND_ASYNC)`
/// always sees `'static` aligned memory (`REQ-086`).
#[cfg(any(windows, test))]
fn chime_wav_words() -> &'static [u16] {
    static WAV_WORDS: OnceLock<Vec<u16>> = OnceLock::new();
    WAV_WORDS.get_or_init(build_chime_wav_words)
}

#[cfg(any(windows, test))]
fn build_chime_wav_words() -> Vec<u16> {
    let sample_rate: u32 = 22_050;
    let duration_secs = 0.28_f32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let data_bytes = (num_samples * 2) as u32;
    let riff_size = 36 + data_bytes;

    let mut bytes = Vec::with_capacity(44 + num_samples * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());

    // Gentle major-third bell harmonic (C6 1046.5 Hz -> E6 1318.5 Hz) with smooth envelope
    let tau = std::f32::consts::TAU;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let attack = (t / 0.012).min(1.0);
        let decay = (-t * 11.0).exp();
        let tone1 = (tau * 1046.5 * t).sin();
        let tone2 = (tau * 1318.5 * (t - 0.07).max(0.0)).sin() * if t > 0.07 { 0.8 } else { 0.0 };
        let sample = ((tone1 * 0.45 + tone2 * 0.55) * attack * decay * 5200.0)
            .clamp(-32767.0, 32767.0) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    if !bytes.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect()
}

#[cfg(windows)]
fn find_current_process_hwnd() -> Option<HWND> {
    struct SearchState {
        pid: u32,
        found: Option<HWND>,
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut SearchState);
        let mut window_pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
        if window_pid == state.pid && IsWindowVisible(hwnd).as_bool() {
            state.found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    let mut state = SearchState {
        pid: std::process::id(),
        found: None,
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_cb),
            LPARAM(&mut state as *mut SearchState as isize),
        );
    }
    state.found
}

#[cfg(windows)]
fn show_windows_toast(title: &str, body: &str) -> windows::core::Result<()> {
    let xml_str = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape_xml(title),
        escape_xml(body)
    );

    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml_str))?;

    let toast = ToastNotification::CreateToastNotification(&doc)?;
    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("AgeMax.Agent"))?;
    notifier.Show(&toast)?;
    Ok(())
}

#[cfg(windows)]
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chime_wav_header_and_cooldown() {
        let words = chime_wav_words();
        assert!(words.len() > 22);
        let first_four: [u8; 4] = [
            words[0].to_le_bytes()[0],
            words[0].to_le_bytes()[1],
            words[1].to_le_bytes()[0],
            words[1].to_le_bytes()[1],
        ];
        assert_eq!(&first_four, b"RIFF");

        let svc = PlatformNotificationService::new();
        let t0 = Instant::now();
        assert!(svc.should_chime_now(t0));
        assert!(!svc.should_chime_now(t0 + Duration::from_millis(500)));
        assert!(svc.should_chime_now(t0 + Duration::from_secs(3)));
    }
}
