//! The error overlay of the Windows adapter (user 2026-11-11: "bolje da se na sredini ekrana pojavi
//! error info overlay"): a window of its own in the middle of the screen, above every window, in
//! QNC's colours (the dark surface and a red band), with an OK button. It is shown without being
//! activated: a program in the tray may not bring a window to the front (Windows showed its own
//! message box minimised), but a topmost window that does not take the keyboard is shown at once.

use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, FrameRect, InvalidateRect, SelectObject, SetBkMode,
    SetTextColor, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, FW_BOLD, FW_NORMAL, PAINTSTRUCT, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetSystemMetrics, GetWindowLongPtrW, LoadCursorW,
    PostMessageW, PostQuitMessage, RegisterClassW, WM_CLOSE, SetWindowLongPtrW, ShowWindow, TranslateMessage, GWLP_USERDATA, IDC_ARROW, MSG, SM_CXSCREEN,
    SM_CYSCREEN, SW_SHOWNOACTIVATE, WM_DESTROY, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const WIDTH: i32 = 560;
const HEIGHT: i32 = 240;
const BAND: i32 = 8;
const MARGIN: i32 = 24;
const BUTTON_W: i32 = 120;
const BUTTON_H: i32 = 34;

/// QNC shell colours (`contracts/ui/shell.layout.json` of QNC): surface, raised, border, text, and
/// QNC's danger red.
fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
}
const SURFACE: (u8, u8, u8) = (17, 24, 39);
const RAISED: (u8, u8, u8) = (31, 41, 55);
const BORDER: (u8, u8, u8) = (55, 65, 81);
const TEXT: (u8, u8, u8) = (229, 231, 235);
const DANGER: (u8, u8, u8) = (220, 38, 38);

struct Alert {
    title: Vec<u16>,
    text: Vec<u16>,
    ok: Vec<u16>,
    hover: bool,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn button() -> RECT {
    RECT { left: WIDTH - MARGIN - BUTTON_W, top: HEIGHT - MARGIN - BUTTON_H, right: WIDTH - MARGIN, bottom: HEIGHT - MARGIN }
}

/// The overlays shown now, by the key their caller gave (one per key).
static SHOWN: std::sync::Mutex<Vec<(String, isize)>> = std::sync::Mutex::new(Vec::new());

/// Shows the overlay of `key` on its own thread (the caller goes on); OK closes it, and so does
/// `close(key)`. An overlay of the same key shown before is closed first.
pub(crate) fn show(key: &str, title: &str, text: &str, ok: &str) {
    close(key);
    let alert = Alert { title: wide(title), text: wide(text), ok: wide(ok), hover: false };
    let key = key.to_string();
    std::thread::spawn(move || run(key, alert));
}

/// Closes the overlay of `key`, when one is shown (what it asked for is done).
pub(crate) fn close(key: &str) {
    let windows: Vec<isize> = match SHOWN.lock() {
        Ok(mut shown) => {
            let (gone, kept): (Vec<_>, Vec<_>) = shown.drain(..).partition(|(known, _)| known == key);
            *shown = kept;
            gone.into_iter().map(|(_, window)| window).collect()
        }
        Err(_) => Vec::new(),
    };
    for window in windows {
        // SAFETY: posting to a window of this process; a window already gone makes the call fail, nothing else.
        unsafe { PostMessageW(window as HWND, WM_CLOSE, 0, 0) };
    }
}

fn run(key: String, alert: Alert) {
    let class: Vec<u16> = "QncAiAlert".encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: Win32 calls on this thread with buffers that live through each call; the window's
    // state is a box owned by the window and freed when it is destroyed.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let mut window_class: WNDCLASSW = std::mem::zeroed();
        window_class.lpfnWndProc = Some(procedure);
        window_class.hInstance = instance;
        window_class.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        window_class.lpszClassName = class.as_ptr();
        RegisterClassW(&window_class);
        let (x, y) = ((GetSystemMetrics(SM_CXSCREEN) - WIDTH) / 2, (GetSystemMetrics(SM_CYSCREEN) - HEIGHT) / 2);
        let window = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            class.as_ptr(),
            WS_POPUP,
            x,
            y,
            WIDTH,
            HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if window.is_null() {
            return;
        }
        SetWindowLongPtrW(window, GWLP_USERDATA, Box::into_raw(Box::new(alert)) as isize);
        if let Ok(mut shown) = SHOWN.lock() {
            shown.push((key, window as isize));
        }
        ShowWindow(window, SW_SHOWNOACTIVATE);
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe fn state<'a>(window: HWND) -> Option<&'a mut Alert> {
    (GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Alert).as_mut()
}

fn inside(rect: &RECT, lparam: LPARAM) -> bool {
    let (x, y) = ((lparam & 0xffff) as i16 as i32, ((lparam >> 16) & 0xffff) as i16 as i32);
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

unsafe fn fill(dc: windows_sys::Win32::Graphics::Gdi::HDC, rect: &RECT, colour: (u8, u8, u8)) {
    let brush = CreateSolidBrush(rgb(colour.0, colour.1, colour.2));
    FillRect(dc, rect, brush);
    DeleteObject(brush);
}

unsafe fn text(dc: windows_sys::Win32::Graphics::Gdi::HDC, words: &[u16], mut rect: RECT, height: i32, bold: bool, format: u32) {
    let face: Vec<u16> = "Segoe UI".encode_utf16().chain(std::iter::once(0)).collect();
    let font = CreateFontW(-height, 0, 0, 0, if bold { FW_BOLD as i32 } else { FW_NORMAL as i32 }, 0, 0, 0, 0, 0, 0, 5, 0, face.as_ptr());
    let old = SelectObject(dc, font);
    DrawTextW(dc, words.as_ptr(), words.len() as i32, &mut rect, format);
    SelectObject(dc, old);
    DeleteObject(font);
}

unsafe extern "system" fn procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(window, &mut paint);
            if let Some(alert) = state(window) {
                let all = RECT { left: 0, top: 0, right: WIDTH, bottom: HEIGHT };
                fill(dc, &all, SURFACE);
                fill(dc, &RECT { left: 0, top: 0, right: WIDTH, bottom: BAND }, DANGER);
                let border = CreateSolidBrush(rgb(DANGER.0, DANGER.1, DANGER.2));
                FrameRect(dc, &all, border);
                DeleteObject(border);
                SetBkMode(dc, TRANSPARENT as i32);
                SetTextColor(dc, rgb(TEXT.0, TEXT.1, TEXT.2));
                text(dc, &alert.title, RECT { left: MARGIN, top: BAND + 14, right: WIDTH - MARGIN, bottom: BAND + 44 }, 20, true, DT_SINGLELINE);
                text(dc, &alert.text, RECT { left: MARGIN, top: BAND + 52, right: WIDTH - MARGIN, bottom: HEIGHT - MARGIN - BUTTON_H - 8 }, 16, false, DT_WORDBREAK);
                let ok = button();
                fill(dc, &ok, if alert.hover { BORDER } else { RAISED });
                let rim = CreateSolidBrush(rgb(BORDER.0, BORDER.1, BORDER.2));
                FrameRect(dc, &ok, rim);
                DeleteObject(rim);
                text(dc, &alert.ok, ok, 16, true, DT_SINGLELINE | DT_CENTER | DT_VCENTER);
            }
            EndPaint(window, &paint);
            0
        }
        WM_MOUSEMOVE => {
            if let Some(alert) = state(window) {
                let hover = inside(&button(), lparam);
                if hover != alert.hover {
                    alert.hover = hover;
                    InvalidateRect(window, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            if inside(&button(), lparam) {
                DestroyWindow(window);
            }
            0
        }
        WM_DESTROY => {
            if let Ok(mut shown) = SHOWN.lock() {
                shown.retain(|(_, known)| *known != window as isize);
            }
            let alert = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Alert;
            if !alert.is_null() {
                SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                drop(Box::from_raw(alert));
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}
