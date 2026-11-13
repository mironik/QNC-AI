//! The window of a QNC-AI form (user 2026-11-13: "moraš formama zadati da se otvore jer se ne
//! vidi dio UI"; "otvorio se najprije manji pa maximizirao"; "to je zakrpa, nije rješenje").
//!
//! Why a form showed a small window first: eframe keeps a wgpu window hidden until its first
//! frame and then shows it; winit shows it with `SW_SHOWNOACTIVATE`, which on Windows shows a
//! window at its normal (restored) size, and only then maximizes it again. So the window's normal
//! size is the screen's work area here: normal and maximized are the same, and nothing smaller is
//! ever shown. The window is maximized too, and the size of its last window is not kept. One block
//! for every form: a form gives only its title and its app.

use eframe::egui;

/// The options of a form's window titled `title`.
pub fn options(title: &str) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default().with_title(title).with_min_inner_size([1100.0, 700.0]).with_maximized(true);
    if let Some((left, top, width, height)) = work_area() {
        viewport = viewport.with_position([left, top]).with_inner_size([width, height]);
    }
    eframe::NativeOptions { renderer: eframe::Renderer::Wgpu, viewport, persist_window: false, ..Default::default() }
}

/// Runs the form's app `app` in its window titled `title`.
pub fn run<A: eframe::App + 'static>(title: &str, app: A) -> eframe::Result<()> {
    eframe::run_native(title, options(title), Box::new(|_| Ok(Box::new(app))))
}

/// The work area of the main screen (without the taskbar), in the points the window is given in:
/// read before the window system makes the program aware of the screen's scale, it is in the
/// screen's logical points. `None` where it is not known (the window is then only maximized).
#[cfg(windows)]
fn work_area() -> Option<(f32, f32, f32, f32)> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA},
    };
    let mut area = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: SPI_GETWORKAREA writes one RECT into the buffer given.
    let read = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut area as *mut RECT).cast(), 0) };
    (read != 0 && area.right > area.left && area.bottom > area.top)
        .then(|| (area.left as f32, area.top as f32, (area.right - area.left) as f32, (area.bottom - area.top) as f32))
}

#[cfg(not(windows))]
fn work_area() -> Option<(f32, f32, f32, f32)> {
    None
}
