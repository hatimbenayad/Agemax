//! Embedded Microsoft WebView2 preview pane with scriptable IPC eval & screenshot capture (Unit 7 §A).

use crate::icons::{icon_label, icon_rich, icon_small, UiIcon};
use agemax_core::layout::PaneId;
use eframe::egui::{self, Color32, Key, Pos2, Rect, Stroke, Vec2};
use serde_json::Value;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserPaneAction {
    SwitchToTerminal(PaneId),
    OpenExternal(String),
}

#[cfg(windows)]
#[derive(Default)]
struct WebViewHostState {
    initializing: bool,
    init_error: Option<String>,
    controller: Option<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller>,
    webview: Option<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2>,
    pending_url: Option<String>,
    last_bounds: Option<(i32, i32, i32, i32)>,
    last_visible: bool,
}

pub struct WebViewSurface {
    pub url: String,
    pub draft_url: String,
    pub init_error: Option<String>,
    #[cfg(windows)]
    host: Rc<RefCell<WebViewHostState>>,
}

impl WebViewSurface {
    pub fn new(initial_url: impl Into<String>) -> Self {
        let normalized = normalize_browser_url(&initial_url.into());
        #[cfg(windows)]
        let host = Rc::new(RefCell::new(WebViewHostState {
            pending_url: Some(normalized.clone()),
            ..Default::default()
        }));

        Self {
            url: normalized.clone(),
            draft_url: normalized,
            init_error: None,
            #[cfg(windows)]
            host,
        }
    }

    pub fn navigate(&mut self, target_url: &str) {
        let clean = normalize_browser_url(target_url);
        self.url = clean.clone();
        self.draft_url = clean.clone();

        #[cfg(windows)]
        {
            let host = self.host.borrow();
            if let Some(wv) = &host.webview {
                let wide = windows::core::HSTRING::from(&clean);
                unsafe {
                    let _ = wv.Navigate(&wide);
                }
            } else {
                drop(host);
                self.host.borrow_mut().pending_url = Some(clean);
            }
        }
    }

    pub fn go_back(&self) {
        #[cfg(windows)]
        if let Some(wv) = &self.host.borrow().webview {
            unsafe {
                let _ = wv.GoBack();
            }
        }
    }

    pub fn go_forward(&self) {
        #[cfg(windows)]
        if let Some(wv) = &self.host.borrow().webview {
            unsafe {
                let _ = wv.GoForward();
            }
        }
    }

    pub fn reload(&self) {
        #[cfg(windows)]
        if let Some(wv) = &self.host.borrow().webview {
            unsafe {
                let _ = wv.Reload();
            }
        }
    }

    pub fn ensure_initialized(&mut self, parent_hwnd: Option<isize>) {
        #[cfg(windows)]
        {
            let Some(hwnd_raw) = parent_hwnd else {
                return;
            };
            {
                let mut host = self.host.borrow_mut();
                if host.controller.is_some() || host.initializing || host.init_error.is_some() {
                    self.init_error = host.init_error.clone();
                    return;
                }
                host.initializing = true;
            }

            let user_data_dir = directories::ProjectDirs::from("com", "agemax", "AgeMax")
                .map(|p| p.data_local_dir().join("webview2"))
                .unwrap_or_else(|| PathBuf::from(".agemax").join("webview2"));
            let _ = std::fs::create_dir_all(&user_data_dir);
            let data_folder =
                windows::core::HSTRING::from(user_data_dir.to_string_lossy().as_ref());

            let host_for_env = Rc::clone(&self.host);
            let hwnd = windows::Win32::Foundation::HWND(hwnd_raw as *mut std::ffi::c_void);

            let env_handler = webview2_com::CreateCoreWebView2EnvironmentCompletedHandler::create(
                Box::new(move |err, env_opt| {
                    if let Err(e) = err {
                        let mut h = host_for_env.borrow_mut();
                        h.initializing = false;
                        h.init_error = Some(format!("WebView2 init failed: {e}"));
                        return Ok(());
                    }
                    let Some(env) = env_opt else {
                        let mut h = host_for_env.borrow_mut();
                        h.initializing = false;
                        h.init_error = Some("WebView2 runtime environment missing".to_string());
                        return Ok(());
                    };

                    let host_for_ctrl = Rc::clone(&host_for_env);
                    let ctrl_handler =
                        webview2_com::CreateCoreWebView2ControllerCompletedHandler::create(
                            Box::new(move |ctrl_err, ctrl_opt| {
                                let mut h = host_for_ctrl.borrow_mut();
                                h.initializing = false;
                                if let Err(e) = ctrl_err {
                                    h.init_error = Some(format!("WebView2 controller error: {e}"));
                                    return Ok(());
                                }
                                let Some(ctrl) = ctrl_opt else {
                                    h.init_error = Some("Missing WebView2 controller".to_string());
                                    return Ok(());
                                };
                                let wv = unsafe { ctrl.CoreWebView2().ok() };
                                if let (Some(wv_ref), Some(url)) = (&wv, h.pending_url.take()) {
                                    let wide = windows::core::HSTRING::from(&url);
                                    unsafe {
                                        let _ = wv_ref.Navigate(&wide);
                                    }
                                }
                                if let Some((l, t, r, b)) = h.last_bounds {
                                    unsafe {
                                        let _ = ctrl.SetBounds(windows::Win32::Foundation::RECT {
                                            left: l,
                                            top: t,
                                            right: r,
                                            bottom: b,
                                        });
                                        let _ = ctrl.SetIsVisible(h.last_visible);
                                    }
                                }
                                h.controller = Some(ctrl);
                                h.webview = wv;
                                Ok(())
                            }),
                        );

                    unsafe {
                        let _ = env.CreateCoreWebView2Controller(hwnd, &ctrl_handler);
                    }
                    Ok(())
                }),
            );

            let res = unsafe {
                webview2_com::Microsoft::Web::WebView2::Win32::CreateCoreWebView2EnvironmentWithOptions(
                    windows::core::PCWSTR::null(),
                    &data_folder,
                    None,
                    &env_handler,
                )
            };
            if let Err(e) = res {
                let mut host = self.host.borrow_mut();
                host.initializing = false;
                let msg = format!("Edge WebView2 runtime unavailable: {e}");
                host.init_error = Some(msg.clone());
                self.init_error = Some(msg);
            }
        }

        #[cfg(not(windows))]
        {
            let _ = parent_hwnd;
            self.init_error = Some("WebView2 preview pane is supported on Windows".to_string());
        }
    }

    pub fn sync_layout_and_visibility(&mut self, visible: bool, bounds_px: (i32, i32, i32, i32)) {
        #[cfg(windows)]
        {
            let mut host = self.host.borrow_mut();
            self.init_error = host.init_error.clone();
            let bounds_changed = host.last_bounds != Some(bounds_px);
            let vis_changed = host.last_visible != visible;
            if !bounds_changed && !vis_changed {
                return;
            }
            host.last_bounds = Some(bounds_px);
            host.last_visible = visible;

            if let Some(ctrl) = &host.controller {
                unsafe {
                    if bounds_changed && bounds_px.2 > bounds_px.0 && bounds_px.3 > bounds_px.1 {
                        let _ = ctrl.SetBounds(windows::Win32::Foundation::RECT {
                            left: bounds_px.0,
                            top: bounds_px.1,
                            right: bounds_px.2,
                            bottom: bounds_px.3,
                        });
                    }
                    if vis_changed {
                        let _ = ctrl.SetIsVisible(visible);
                    }
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (visible, bounds_px);
        }
    }

    pub fn eval_js(&self, script: &str, reply: oneshot::Sender<Value>) {
        #[cfg(windows)]
        {
            let host = self.host.borrow();
            let Some(wv) = host.webview.clone() else {
                let _ = reply.send(serde_json::json!({
                    "error": "WebView2 page is still initializing or unavailable"
                }));
                return;
            };
            drop(host);

            let tx = RefCell::new(Some(reply));
            let handler = webview2_com::ExecuteScriptCompletedHandler::create(Box::new(
                move |err, result_json| {
                    if let Some(sender) = tx.borrow_mut().take() {
                        match err {
                            Ok(()) => {
                                let parsed: Value = serde_json::from_str(&result_json)
                                    .unwrap_or(Value::String(result_json));
                                let _ = sender.send(serde_json::json!({ "result": parsed }));
                            }
                            Err(e) => {
                                let _ = sender.send(serde_json::json!({ "error": e.to_string() }));
                            }
                        }
                    }
                    Ok(())
                },
            ));

            let wide = windows::core::HSTRING::from(script);
            if let Err(e) = unsafe { wv.ExecuteScript(&wide, &handler) } {
                tracing::debug!("WebView2 ExecuteScript failed: {e}");
            }
        }
        #[cfg(not(windows))]
        {
            let _ = script;
            let _ = reply.send(serde_json::json!({ "error": "WebView2 unsupported on this OS" }));
        }
    }

    pub fn capture_screenshot(&self, pane_id: PaneId, reply: oneshot::Sender<Value>) {
        #[cfg(windows)]
        {
            let host = self.host.borrow();
            let Some(wv) = host.webview.clone() else {
                let _ = reply.send(serde_json::json!({
                    "error": "WebView2 page is still initializing or unavailable"
                }));
                return;
            };
            drop(host);

            let stream = unsafe {
                windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal(
                    windows::Win32::Foundation::HGLOBAL::default(),
                    true,
                )
            };
            let Ok(stream) = stream else {
                let _ = reply.send(serde_json::json!({ "error": "Failed creating COM stream" }));
                return;
            };

            let stream_for_cb = stream.clone();
            let tx = RefCell::new(Some(reply));
            let handler =
                webview2_com::CapturePreviewCompletedHandler::create(Box::new(move |err| {
                    if let Some(sender) = tx.borrow_mut().take() {
                        if let Err(e) = err {
                            let _ = sender.send(serde_json::json!({ "error": e.to_string() }));
                            return Ok(());
                        }

                        let bytes = unsafe { read_hglobal_stream(&stream_for_cb) };
                        let captures_dir =
                            directories::ProjectDirs::from("com", "agemax", "AgeMax")
                                .map(|p| p.data_local_dir().join("captures"))
                                .unwrap_or_else(|| PathBuf::from(".agemax").join("captures"));
                        let _ = std::fs::create_dir_all(&captures_dir);
                        let out_path = captures_dir.join(format!("{}.png", pane_id.0));
                        match std::fs::write(&out_path, &bytes) {
                            Ok(()) => {
                                let _ = sender.send(serde_json::json!({
                                    "path": out_path.to_string_lossy(),
                                    "bytes": bytes.len(),
                                }));
                            }
                            Err(e) => {
                                let _ = sender.send(serde_json::json!({ "error": e.to_string() }));
                            }
                        }
                    }
                    Ok(())
                }));

            let fmt = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
            if let Err(e) = unsafe { wv.CapturePreview(fmt, &stream, &handler) } {
                tracing::debug!("WebView2 CapturePreview call failed: {e}");
            }
        }
        #[cfg(not(windows))]
        {
            let _ = pane_id;
            let _ = reply.send(serde_json::json!({ "error": "WebView2 unsupported on this OS" }));
        }
    }
}

#[cfg(windows)]
impl Drop for WebViewSurface {
    fn drop(&mut self) {
        if let Some(ctrl) = self.host.borrow_mut().controller.take() {
            unsafe {
                let _ = ctrl.SetIsVisible(false);
                let _ = ctrl.Close();
            }
        }
    }
}

#[cfg(windows)]
unsafe fn read_hglobal_stream(stream: &windows::Win32::System::Com::IStream) -> Vec<u8> {
    let Ok(hglobal) = windows::Win32::System::Com::StructuredStorage::GetHGlobalFromStream(stream)
    else {
        return Vec::new();
    };
    let size = windows::Win32::System::Memory::GlobalSize(hglobal);
    if size == 0 {
        return Vec::new();
    }
    let ptr = windows::Win32::System::Memory::GlobalLock(hglobal);
    if ptr.is_null() {
        return Vec::new();
    }
    let slice = std::slice::from_raw_parts(ptr as *const u8, size);
    let data = slice.to_vec();
    let _ = windows::Win32::System::Memory::GlobalUnlock(hglobal);
    data
}

pub fn normalize_browser_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "http://localhost:3000".to_string();
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("data:")
        || trimmed.starts_with("about:")
    {
        trimmed.to_string()
    } else if trimmed.starts_with("localhost") || trimmed.starts_with("127.0.0.1") {
        format!("http://{trimmed}")
    } else {
        format!("https://{trimmed}")
    }
}

/// Render the embedded browser pane's top URL toolbar and WebView2 / fallback content region.
pub fn render_browser_pane(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pane_id: PaneId,
    body_rect: Rect,
    surface: &mut WebViewSurface,
    allow_native_overlay: bool,
    host_hwnd: Option<isize>,
) -> Option<BrowserPaneAction> {
    let mut action = None;
    let bar_h = 30.0;
    let toolbar_rect = Rect::from_min_max(
        body_rect.min,
        Pos2::new(
            body_rect.max.x,
            (body_rect.min.y + bar_h).min(body_rect.max.y),
        ),
    );
    let web_rect = Rect::from_min_max(
        Pos2::new(body_rect.min.x, toolbar_rect.max.y),
        body_rect.max,
    );

    ui.painter()
        .rect_filled(toolbar_rect, 0.0, Color32::from_rgb(24, 28, 40));
    ui.painter().line_segment(
        [
            Pos2::new(toolbar_rect.min.x, toolbar_rect.max.y),
            Pos2::new(toolbar_rect.max.x, toolbar_rect.max.y),
        ],
        Stroke::new(1.0, Color32::from_rgb(42, 48, 66)),
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("browser_toolbar", pane_id.0))
            .max_rect(toolbar_rect.shrink2(Vec2::new(6.0, 3.0))),
        |ui| {
            ui.horizontal(|ui| {
                if ui
                    .small_button(icon_rich(UiIcon::ArrowLeft, 11.0, None))
                    .on_hover_text("Back")
                    .clicked()
                {
                    surface.go_back();
                }
                if ui
                    .small_button(icon_rich(UiIcon::ArrowRight, 11.0, None))
                    .on_hover_text("Forward")
                    .clicked()
                {
                    surface.go_forward();
                }
                if ui
                    .small_button(icon_rich(UiIcon::RotateCw, 11.0, None))
                    .on_hover_text("Reload")
                    .clicked()
                {
                    surface.reload();
                }

                let nav_url_w = (ui.available_width() - 190.0).max(100.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut surface.draft_url)
                        .desired_width(nav_url_w)
                        .text_color(Color32::WHITE),
                );
                let submit = (resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    || ui.small_button("Go").clicked();
                if submit {
                    let next = surface.draft_url.clone();
                    surface.navigate(&next);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(icon_small(UiIcon::ArrowLeft, "Terminal"))
                        .on_hover_text("Switch this pane back to terminal")
                        .clicked()
                    {
                        action = Some(BrowserPaneAction::SwitchToTerminal(pane_id));
                    }
                    if ui
                        .small_button(icon_small(UiIcon::ArrowUpRight, "Ext"))
                        .on_hover_text("Open URL in system default browser")
                        .clicked()
                    {
                        action = Some(BrowserPaneAction::OpenExternal(surface.url.clone()));
                    }
                });
            });
        },
    );

    surface.ensure_initialized(host_hwnd);

    let ppp = ctx.pixels_per_point();
    let bounds_px = (
        (web_rect.min.x * ppp).round() as i32,
        (web_rect.min.y * ppp).round() as i32,
        (web_rect.max.x * ppp).round() as i32,
        (web_rect.max.y * ppp).round() as i32,
    );
    let can_show = allow_native_overlay && web_rect.width() > 32.0 && web_rect.height() > 32.0;
    surface.sync_layout_and_visibility(can_show, bounds_px);

    if let Some(err) = &surface.init_error {
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt(("browser_fallback", pane_id.0))
                .max_rect(web_rect.shrink(16.0)),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(icon_label(
                        UiIcon::Globe,
                        "Embedded WebView2 Preview Unavailable",
                        14.0,
                        Some(Color32::from_rgb(245, 175, 45)),
                    ));
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(err)
                            .size(11.5)
                            .color(Color32::from_rgb(175, 185, 205)),
                    );
                    ui.add_space(10.0);
                    if ui
                        .button(format!("Open {} externally", surface.url))
                        .clicked()
                    {
                        action = Some(BrowserPaneAction::OpenExternal(surface.url.clone()));
                    }
                });
            },
        );
    }

    action
}
