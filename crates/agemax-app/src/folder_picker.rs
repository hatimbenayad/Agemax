use crossbeam_channel::Receiver;
use eframe::egui;
use std::path::{Path, PathBuf};

/// Platform-agnostic folder chooser dialog abstraction.
pub trait FolderPicker: Send + Sync {
    /// Open a native folder selection dialog starting at `initial_dir` if provided.
    fn pick_folder(&self, initial_dir: Option<&Path>) -> Option<PathBuf>;
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
    fn pick_folder(&self, initial_dir: Option<&Path>) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            pick_folder_windows(initial_dir)
        }

        #[cfg(not(windows))]
        {
            let _ = initial_dir;
            None
        }
    }
}

/// Spawn the native folder picker on a dedicated STA thread so UI rendering never blocks.
pub fn spawn_folder_picker_async(
    initial_dir: Option<PathBuf>,
    ctx: egui::Context,
) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::Builder::new()
        .name("agemax-folder-picker".to_string())
        .spawn(move || {
            let picker = PlatformFolderPicker::new();
            let chosen = picker.pick_folder(initial_dir.as_deref());
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
fn pick_folder_windows(initial_dir: Option<&Path>) -> Option<PathBuf> {
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

        if dialog.Show(None).is_err() {
            return None;
        }

        let result_item = dialog.GetResult().ok()?;
        let display_ptr = result_item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path_str = display_ptr.to_string().ok();
        CoTaskMemFree(Some(display_ptr.as_ptr() as *const _));

        path_str.map(PathBuf::from)
    }
}
