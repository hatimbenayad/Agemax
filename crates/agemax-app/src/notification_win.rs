use agemax_core::notification::NotificationItem;
use agemax_core::status::AgentStatus;

#[cfg(windows)]
use windows::core::HSTRING;
#[cfg(windows)]
use windows::Data::Xml::Dom::XmlDocument;
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    FlashWindowEx, GetForegroundWindow, FLASHWINFO, FLASHW_TIMERNOFG, FLASHW_TRAY,
};
#[cfg(windows)]
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

pub trait NotificationService: Send + Sync {
    fn show_notification(&self, item: &NotificationItem);
    fn flash_window(&self);
}

pub struct PlatformNotificationService;

impl PlatformNotificationService {
    pub fn new() -> Self {
        Self
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

            // Attempt native UWP / WinRT Toast notification
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
            let hwnd = GetForegroundWindow();
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
