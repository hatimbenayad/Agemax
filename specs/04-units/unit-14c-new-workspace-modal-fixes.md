# Unit 14c: NewWorkspaceModal Responsiveness, Owner HWND & Overlay Fixes

## 1. Scope
Fix the five verified UI/threading defects in `NewWorkspaceModal` and workspace creation (`REQ-095`), without adding any per-workspace folder-lock checkbox (deferred to Unit 14b):
1. **Async WSL detection**: Move `detect_wsl_distros()` off `NewWorkspaceModal::open()` onto a background thread (`agemax-wsl-detect`), triggered lazily when the WSL tab is active and polled via a `crossbeam_channel::Receiver<Vec<String>>`.
2. **Owned Windows `IFileOpenDialog`**: Add `spawn_folder_picker_with_owner(initial_dir, owner_hwnd, ctx)` in `crates/agemax-app/src/folder_picker.rs` and pass `host_hwnd: Option<isize>` from `NewWorkspaceModal::show` so `dialog.Show(owner)` stays modal above the Age Max window.
3. **Debounced local path validation**: Cache `cached_path_error: Option<String>` in `NewWorkspaceModal` and only call `validate_workspace_path` on modal open, `path_input` edit (`changed()`), async picker return, `allowed_root` change, or submit—never on idle frames.
4. **Non-blocking workspace submit**: Remove synchronous `detect_git_branch(&req.cwd)` from `create_workspace_with_backend` in `crates/agemax-app/src/main.rs` so `GitStatsPoller` populates `git_branch` asynchronously off the UI thread.
5. **Clear Home/Grid overlays on submit**: Clear `show_welcome_view` and `show_grid_view` when `create_workspace_with_backend` succeeds.

## 2. Requirements Traceability
- **REQ-095** (`specs/01-requirements.md`).

## 3. Design & Deliverables
- `crates/agemax-app/src/folder_picker.rs`:
  - `spawn_folder_picker_with_owner(initial_dir: Option<PathBuf>, owner_hwnd: Option<isize>, ctx: egui::Context) -> Receiver<Option<PathBuf>>`.
  - Keep `spawn_folder_picker_async` delegating to `spawn_folder_picker_with_owner(initial_dir, None, ctx)`.
  - On Windows, pass `owner_hwnd.filter(|&h| h != 0)` as `HWND` to `dialog.Show(owner)`.
- `crates/agemax-app/src/chrome_ui.rs`:
  - `NewWorkspaceModal` fields for async WSL detection (`wsl_rx`, `wsl_detect_started`) and cached path validation (`cached_path_error`, `last_validated_root`, `needs_path_revalidation`).
  - `NewWorkspaceModal::show` accepts `host_hwnd: Option<isize>`.
- `crates/agemax-app/src/main.rs`:
  - Pass `self.host_hwnd` to `new_workspace_modal.show(...)`.
  - Remove synchronous `detect_git_branch` from `create_workspace_with_backend` and clear `show_welcome_view` / `show_grid_view` on success.
- Unit tests in `chrome_ui.rs` and `folder_picker.rs` proving async WSL state, debounced validation caching, and owner HWND handling.
