//! Opt-in Windows 11 DWM Mica backdrop & graceful Windows 10 opaque fallback (`REQ-089`).

/// Tracks whether the DWM system backdrop is currently enabled on `hwnd`.
#[derive(Debug, Default)]
pub struct BackdropController {
    last_applied: Option<bool>,
    pub mica_active: bool,
}

impl BackdropController {
    /// Apply or remove Windows 11 Mica backdrop (`DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW`).
    /// Returns `true` when Mica is currently active on a compatible Windows DWM host.
    pub fn sync(&mut self, host_hwnd: Option<isize>, enable: bool) -> bool {
        let Some(raw_hwnd) = host_hwnd else {
            return false;
        };
        if self.last_applied == Some(enable) {
            return self.mica_active;
        }
        self.last_applied = Some(enable);
        self.mica_active = apply_dwm_mica_impl(raw_hwnd, enable);
        self.mica_active
    }
}

#[cfg(windows)]
fn apply_dwm_mica_impl(raw_hwnd: isize, enable: bool) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_NONE,
        DWMWA_SYSTEMBACKDROP_TYPE,
    };
    use windows::Win32::UI::Controls::MARGINS;

    let hwnd = HWND(raw_hwnd as *mut std::ffi::c_void);
    let margins = if enable {
        MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        }
    } else {
        MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 0,
        }
    };
    let backdrop = if enable {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };

    unsafe {
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        let res = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&backdrop) as u32,
        );
        enable && res.is_ok()
    }
}

#[cfg(not(windows))]
fn apply_dwm_mica_impl(_raw_hwnd: isize, _enable: bool) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backdrop_controller_handles_missing_hwnd_gracefully() {
        let mut ctrl = BackdropController::default();
        assert!(!ctrl.sync(None, true));
        assert!(!ctrl.mica_active);
    }
}
