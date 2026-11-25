use crossbeam_channel::Receiver;
use eframe::egui;
use std::path::{Path, PathBuf};

/// Platform-agnostic folder chooser dialog abstraction.
pub trait FolderPicker: Send + Sync {
    /// Open a native folder selection dialog starting at `initial_dir` if provided.
    fn pick_folder(&self, initial_dir: Option<&Path>) -> Option<PathBuf> {
        self.pick_folder_with_owner(initial_dir, None)
    }

    /// Open a native folder selection dialog owned by `owner_hwnd` if provided.
    fn pick_folder_with_owner(
        &self,
        initial_dir: Option<&Path>,
        owner_hwnd: Option<isize>,
    ) -> Option<PathBuf>;
}

/// Native platform folder picker (`IFileOpenDialog` on Windows).
#[derive(Debug, Default)]
pub struct PlatformFolderPicker;

impl PlatformFolderPicker {
    pub fn new() -> Self {
        Self
    }
}

impl FolderPicker for PlatformFolderPicker {
    fn pick_folder_with_owner(
        &self,
        initial_dir: Option<&Path>,
        owner_hwnd: Option<isize>,
    ) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            pick_folder_windows(initial_dir, owner_hwnd)
        }

        #[cfg(not(windows))]
        {
            let _ = (initial_dir, owner_hwnd);
            None
        }
    }
}

pub(crate) fn normalize_owner_hwnd(owner_hwnd: Option<isize>) -> Option<isize> {
    owner_hwnd.filter(|&h| h != 0)
}

/// Spawn the native folder picker on a dedicated STA thread so UI rendering never blocks.
pub fn spawn_folder_picker_async(
    initial_dir: Option<PathBuf>,
    ctx: egui::Context,
) -> Receiver<Option<PathBuf>> {
    spawn_folder_picker_with_owner(initial_dir, None, ctx)
}

/// Spawn the native folder picker owned by `owner_hwnd` on a dedicated STA thread.
pub fn spawn_folder_picker_with_owner(
    initial_dir: Option<PathBuf>,
    owner_hwnd: Option<isize>,
    ctx: egui::Context,
) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::Builder::new()
        .name("agemax-folder-picker".to_string())
        .spawn(move || {
            let picker = PlatformFolderPicker::new();
            let chosen = match normalize_owner_hwnd(owner_hwnd) {
                Some(hwnd) => picker.pick_folder_with_owner(initial_dir.as_deref(), Some(hwnd)),
                None => picker.pick_folder(initial_dir.as_deref()),
            };
            let _ = tx.send(chosen);
            ctx.request_repaint();
        })
        .ok();
    rx
}

#[cfg(windows)]
struct ComApartmentGuard(bool);

#[cfg(windows)]
impl Drop for ComApartmentGuard {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: Balanced with a successful CoInitializeEx on the same STA thread.
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

#[cfg(windows)]
fn pick_folder_windows(initial_dir: Option<&Path>, owner_hwnd: Option<isize>) -> Option<PathBuf> {
    use windows::core::HSTRING;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName,
        FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    // SAFETY: Running on a dedicated single-threaded apartment worker thread cleanly separated from UI.
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let _guard = ComApartmentGuard(hr.is_ok());

        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).ok()?;

        let mut opts = dialog.GetOptions().ok()?;
        opts |= FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_NOCHANGEDIR;
        dialog.SetOptions(opts).ok()?;
        let _ = dialog.SetTitle(&HSTRING::from("Select Project Folder for Workspace"));

        if let Some(dir) = initial_dir.filter(|p| p.exists()) {
            let dir_hstring = HSTRING::from(dir.as_os_str());
            if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&dir_hstring, None) {
                let _ = dialog.SetFolder(&item);
            }
        }

        let owner = normalize_owner_hwnd(owner_hwnd)
            .map(|h| windows::Win32::Foundation::HWND(h as *mut std::ffi::c_void));
        if dialog.Show(owner).is_err() {
            return None;
        }

        let result_item = dialog.GetResult().ok()?;
        let display_ptr = result_item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path_str = display_ptr.to_string().ok();
        CoTaskMemFree(Some(display_ptr.as_ptr() as *const _));

        path_str.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_owner_hwnd;

    #[test]
    fn test_normalize_owner_hwnd() {
        assert_eq!(normalize_owner_hwnd(None), None);
        assert_eq!(normalize_owner_hwnd(Some(0)), None);
        assert_eq!(normalize_owner_hwnd(Some(-1)), Some(-1));
        assert_eq!(normalize_owner_hwnd(Some(0x1234)), Some(0x1234));
    }
}
