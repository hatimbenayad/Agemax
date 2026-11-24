//! Single-instance CLI argument parsing and Named Pipe activation helper (REQ-079).

use crate::client::IpcClient;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Command-line options accepted by `agemax-app.exe`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchArgs {
    pub cwd: Option<PathBuf>,
    pub pipe: Option<String>,
}

impl LaunchArgs {
    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let mut iter = args.into_iter().skip(1).map(Into::into);
        let mut out = Self::default();

        while let Some(arg) = iter.next() {
            let s = arg.to_string_lossy();
            match s.as_ref() {
                "--cwd" | "--dir" => {
                    if let Some(val) = iter.next() {
                        let candidate = PathBuf::from(val);
                        if !candidate.as_os_str().is_empty() {
                            out.cwd = Some(candidate);
                        }
                    }
                }
                "--pipe" => {
                    if let Some(val) = iter.next() {
                        let p = val.to_string_lossy().trim().to_string();
                        if !p.is_empty() {
                            out.pipe = Some(p);
                        }
                    }
                }
                flag if flag.starts_with('-') => {}
                positional => {
                    if out.cwd.is_none() && !positional.trim().is_empty() {
                        out.cwd = Some(PathBuf::from(arg));
                    }
                }
            }
        }

        out
    }
}

/// Grant foreground window permission to an already running instance on Windows.
fn allow_foreground_handoff() {
    #[cfg(windows)]
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
            windows::Win32::UI::WindowsAndMessaging::ASFW_ANY,
        );
    }
}

/// Attempt to activate an already-running Age Max instance listening on `pipe_name`.
///
/// Returns `true` if a running instance accepted `app.activate` and acknowledged activation,
/// meaning the caller should exit cleanly instead of opening a duplicate window.
pub fn try_activate_existing_instance(pipe_name: &str, cwd: Option<&Path>) -> bool {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    allow_foreground_handoff();
    let client = IpcClient::new(pipe_name);
    let cwd_param = cwd.map(|p| p.to_string_lossy().to_string());
    let params = serde_json::json!({ "cwd": cwd_param });

    rt.block_on(async {
        for attempt in 0..4 {
            match tokio::time::timeout(
                std::time::Duration::from_millis(250),
                client.call("app.activate", params.clone()),
            )
            .await
            {
                Ok(Ok(val)) => {
                    return val
                        .get("activated")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                }
                Ok(Err(_)) if attempt < 3 => {
                    // Retry briefly in case the named pipe instance is momentarily busy.
                    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                }
                _ => return false,
            }
        }
        false
    })
}
