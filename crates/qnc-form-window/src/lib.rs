//! The window of a QNC-AI form (user 2026-11-13: "moraš formama zadati da se otvore jer se ne
//! vidi dio UI"; "otvorio se najprije manji pa maximizirao"; "to je zakrpa, nije rješenje"; "i dalje
//! se pokaže manji prazan prozor a onda maximize").
//!
//! Measured (every window of the form's process, without a pause): a window asked to be maximized
//! when it is made is shown by Windows at once (`SW_MAXIMIZE` shows a window), eframe hides it
//! until its first frame, and winit shows it again with `SW_SHOWNOACTIVATE`, at its normal size and
//! empty, before maximizing it once more: shown, hidden, shown small, hidden, shown. So the window
//! is not made maximized. It is made at the size and place of the screen's work area (the screen
//! without the taskbar), eframe shows it once, painted, and only then is it maximized: it is
//! already that size, so nothing is seen to change. The size of its last window is not kept. One
//! block for every form: a form gives only its title and its app.

use eframe::egui;

/// The options of a form's window titled `title`.
pub fn options(title: &str) -> eframe::NativeOptions {
    // QNC's own title bar (`qnc-title-bar`), with the archive search and the settings: no system frame.
    let viewport = qnc_title_bar::frameless(egui::ViewportBuilder::default().with_title(title).with_min_inner_size([1100.0, 700.0]));
    let viewport = match maximized_place() {
        Some((left, top, width, height)) => viewport.with_position([left, top]).with_inner_size([width, height]),
        // Where the work area is not known the window is made maximized.
        None => viewport.with_maximized(true),
    };
    eframe::NativeOptions { renderer: eframe::Renderer::Wgpu, viewport, persist_window: false, ..Default::default() }
}

/// Runs the form's app `app` in its window titled `title`.
pub fn run<A: eframe::App + 'static>(title: &str, app: A) -> eframe::Result<()> {
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_default();
    let framed = qnc_title_bar::Framed { bar: qnc_title_bar::TitleBar::new(title, root), app };
    eframe::run_native(title, options(title), Box::new(|_| Ok(Box::new(Maximized { app: framed, frames: 0 }))))
}

/// The frame after which the window, shown by eframe after its first frame, is maximized.
const MAXIMIZE_AFTER: u32 = 2;

/// A form's app in its window: maximized once eframe has shown it.
struct Maximized<A> {
    app: A,
    frames: u32,
}

impl<A: eframe::App> eframe::App for Maximized<A> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.frames <= MAXIMIZE_AFTER {
            self.frames += 1;
            if self.frames == MAXIMIZE_AFTER {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            } else {
                ctx.request_repaint();
            }
        }
        self.app.update(ctx, frame);
    }
}

/// The place of the window as Windows gives it when maximized (user 2026-11-13: "uskladi normalni
/// prozor s maksimiziranim"): its outer top-left and its inner size. The window has no system
/// frame (QNC's own title bar, 2026-11-15), so maximized it is the work area (the screen without
/// the taskbar) itself; the normal window is given the same, so maximizing it changes nothing seen.
/// The work area is read at the screen's real scale, in its pixels, then given in points. `None`
/// where it is not known.
#[cfg(windows)]
fn maximized_place() -> Option<(f32, f32, f32, f32)> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::{
            HiDpi::{GetDpiForSystem, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA},
        },
    };
    let mut area = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: this thread reads in the screen's pixels for these calls only and is put back; the
    // call writes one RECT into the buffer given.
    let (read, dpi) = unsafe {
        let before = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let dpi = GetDpiForSystem();
        let read = SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut area as *mut RECT).cast(), 0);
        SetThreadDpiAwarenessContext(before);
        (read, dpi)
    };
    let (width, height) = (area.right - area.left, area.bottom - area.top);
    if read == 0 || dpi == 0 || width <= 0 || height <= 0 {
        return None;
    }
    let points = |pixels: i32| pixels as f32 * 96.0 / dpi as f32;
    Some((points(area.left), points(area.top), points(width), points(height)))
}

#[cfg(not(windows))]
fn maximized_place() -> Option<(f32, f32, f32, f32)> {
    None
}
