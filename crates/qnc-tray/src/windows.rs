//! The Windows adapter: a hidden message window on its own thread owns the shell's notify icon.
//! One tray per process (the window procedure reaches its state through one static).

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{GetLastError, CloseHandle, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
    HWND_MESSAGE, IDI_APPLICATION, MF_GRAYED, MF_STRING, MSG, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CLOSE, WM_DESTROY,
    WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
};

const WM_TRAY: u32 = WM_APP + 1;
const WM_UPDATE: u32 = WM_APP + 2;

struct Shared {
    events: Sender<TrayEvent>,
    menu: Vec<MenuItem>,
    tip: String,
    notice: Option<(String, String)>,
}

use crate::{MenuItem, TrayEvent};

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

pub(crate) struct Tray {
    window: isize,
    thread: Option<std::thread::JoinHandle<()>>,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn copy_into(target: &mut [u16], text: &str) {
    let units: Vec<u16> = text.encode_utf16().take(target.len() - 1).collect();
    target[..units.len()].copy_from_slice(&units);
    target[units.len()] = 0;
}

fn icon_data(window: HWND) -> NOTIFYICONDATAW {
    // SAFETY: a plain C struct; zero is a valid start for every field.
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = window;
    data.uID = 1;
    data
}

impl Tray {
    pub(crate) fn start(tip: &str, menu: Vec<MenuItem>) -> Result<(Self, Receiver<TrayEvent>), String> {
        let (send, events) = mpsc::channel();
        {
            let mut shared = SHARED.lock().map_err(|_| "tray state")?;
            if shared.is_some() {
                return Err("one tray per process".into());
            }
            *shared = Some(Shared { events: send, menu, tip: tip.into(), notice: None });
        }
        let (made, window) = mpsc::channel();
        let thread = std::thread::spawn(move || run(made));
        match window.recv() {
            Ok(Ok(window)) => Ok((Self { window, thread: Some(thread) }, events)),
            Ok(Err(error)) => {
                let _ = thread.join();
                *SHARED.lock().unwrap_or_else(|poison| poison.into_inner()) = None;
                Err(error)
            }
            Err(_) => Err("the tray thread ended".into()),
        }
    }

    fn update(&self) {
        // SAFETY: posting to a window of this process; a gone window makes the call fail, nothing else.
        unsafe { PostMessageW(self.window as HWND, WM_UPDATE, 0, 0) };
    }

    pub(crate) fn notify(&self, title: &str, text: &str) {
        if let Ok(mut shared) = SHARED.lock() {
            if let Some(shared) = shared.as_mut() {
                shared.notice = Some((title.into(), text.into()));
            }
        }
        self.update();
    }

    pub(crate) fn set_menu(&self, menu: Vec<MenuItem>) {
        if let Ok(mut shared) = SHARED.lock() {
            if let Some(shared) = shared.as_mut() {
                shared.menu = menu;
            }
        }
    }

    pub(crate) fn set_tip(&self, tip: &str) {
        if let Ok(mut shared) = SHARED.lock() {
            if let Some(shared) = shared.as_mut() {
                shared.tip = tip.into();
            }
        }
        self.update();
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: as in `update`.
        unsafe { PostMessageW(self.window as HWND, WM_CLOSE, 0, 0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        *SHARED.lock().unwrap_or_else(|poison| poison.into_inner()) = None;
    }
}

fn run(made: Sender<Result<isize, String>>) {
    let class = wide("QncAiTrayWindow");
    // SAFETY: Win32 calls with buffers that live through each call; the window and its icon
    // belong to this thread, which runs their message loop until the window is destroyed.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let mut window_class: WNDCLASSW = std::mem::zeroed();
        window_class.lpfnWndProc = Some(procedure);
        window_class.hInstance = instance;
        window_class.lpszClassName = class.as_ptr();
        RegisterClassW(&window_class);
        let window = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, HWND_MESSAGE, std::ptr::null_mut(), instance, std::ptr::null());
        if window.is_null() {
            let _ = made.send(Err(format!("the tray window was not made ({})", GetLastError())));
            return;
        }
        let mut data = icon_data(window);
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = WM_TRAY;
        data.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        let tip = SHARED.lock().ok().and_then(|shared| shared.as_ref().map(|shared| shared.tip.clone())).unwrap_or_default();
        copy_into(&mut data.szTip, &tip);
        if Shell_NotifyIconW(NIM_ADD, &data) == 0 {
            DestroyWindow(window);
            let _ = made.send(Err("the shell did not take the tray icon".into()));
            return;
        }
        let _ = made.send(Ok(window as isize));
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn send(event: TrayEvent) {
    if let Ok(shared) = SHARED.lock() {
        if let Some(shared) = shared.as_ref() {
            let _ = shared.events.send(event);
        }
    }
}

unsafe extern "system" fn procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_TRAY => {
            match lparam as u32 {
                // A click with either button shows the menu (user 2026-11-11: "klik na tray moze pokazati tray izbornik").
                WM_LBUTTONUP | WM_RBUTTONUP => {
                    if let Some(id) = menu(window) {
                        send(TrayEvent::Menu(id));
                    }
                }
                _ => {}
            }
            0
        }
        WM_UPDATE => {
            let (tip, notice) = match SHARED.lock() {
                Ok(mut shared) => match shared.as_mut() {
                    Some(shared) => (shared.tip.clone(), shared.notice.take()),
                    None => return 0,
                },
                Err(_) => return 0,
            };
            let mut data = icon_data(window);
            data.uFlags = NIF_TIP;
            copy_into(&mut data.szTip, &tip);
            if let Some((title, text)) = notice {
                data.uFlags |= NIF_INFO;
                copy_into(&mut data.szInfoTitle, &title);
                copy_into(&mut data.szInfo, &text);
                data.dwInfoFlags = NIIF_INFO;
            }
            Shell_NotifyIconW(NIM_MODIFY, &data);
            0
        }
        WM_CLOSE => {
            DestroyWindow(window);
            0
        }
        WM_DESTROY => {
            let data = icon_data(window);
            Shell_NotifyIconW(NIM_DELETE, &data);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

/// The menu at the cursor; the id of the chosen item.
unsafe fn menu(window: HWND) -> Option<u32> {
    let items = SHARED.lock().ok()?.as_ref()?.menu.clone();
    let popup = CreatePopupMenu();
    for item in &items {
        let text = wide(&item.text);
        let flags = if item.enabled { MF_STRING } else { MF_STRING | MF_GRAYED };
        AppendMenuW(popup, flags, item.id as usize, text.as_ptr());
    }
    let mut at = POINT { x: 0, y: 0 };
    GetCursorPos(&mut at);
    // The menu closes when the user clicks elsewhere only if the window is in front.
    SetForegroundWindow(window);
    let chosen = TrackPopupMenu(popup, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, at.x, at.y, 0, window, std::ptr::null());
    DestroyMenu(popup);
    (chosen > 0).then_some(chosen as u32)
}

/// The error overlay in the middle of the screen (`alert.rs`).
pub(crate) fn alert(key: &str, title: &str, text: &str, ok: &str) {
    crate::alert::show(key, title, text, ok);
}

pub(crate) fn info(key: &str, title: &str, text: &str, ok: &str, close_after: std::time::Duration) {
    crate::alert::info(key, title, text, ok, close_after);
}

pub(crate) fn close_alert(key: &str) {
    crate::alert::close(key);
}

/// A question with Yes and No, above every window, on its own thread; the answer comes back on
/// the channel (Yes is `true`).
pub(crate) fn confirm(title: &str, text: &str) -> std::sync::mpsc::Receiver<bool> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO};
    let (send, answer) = std::sync::mpsc::channel();
    let (title, text) = (wide(title), wide(text));
    std::thread::spawn(move || {
        // SAFETY: both texts are NUL-terminated UTF-16 that live until the box is closed.
        let chosen = unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 | MB_TOPMOST | MB_SETFOREGROUND)
        };
        let _ = send.send(chosen == IDYES);
    });
    answer
}

/// A question with Yes, No and Cancel (Yes chosen when Enter is pressed), above every window, on
/// its own thread; the answer comes back on the channel.
pub(crate) fn ask(title: &str, text: &str) -> std::sync::mpsc::Receiver<crate::Answer> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDNO, IDYES, MB_ICONQUESTION, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNOCANCEL};
    let (send, answer) = std::sync::mpsc::channel();
    let (title, text) = (wide(title), wide(text));
    std::thread::spawn(move || {
        // SAFETY: both texts are NUL-terminated UTF-16 that live until the box is closed.
        let chosen = unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_YESNOCANCEL | MB_ICONQUESTION | MB_TOPMOST | MB_SETFOREGROUND) };
        let _ = send.send(match chosen {
            IDYES => crate::Answer::Yes,
            IDNO => crate::Answer::No,
            _ => crate::Answer::Cancel,
        });
    });
    answer
}

pub(crate) struct OneCopy(HANDLE);

// SAFETY: the handle is only closed, once, by its owner.
unsafe impl Send for OneCopy {}

impl OneCopy {
    pub(crate) fn take(name: &str) -> Result<Option<Self>, String> {
        let name = wide(&format!("Local\\{name}"));
        // SAFETY: a named mutex of this session; the name lives through the call.
        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
            if handle.is_null() {
                return Err(format!("one copy: {}", GetLastError()));
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return Ok(None);
            }
            Ok(Some(Self(handle)))
        }
    }
}

impl Drop for OneCopy {
    fn drop(&mut self) {
        // SAFETY: the handle this copy made.
        unsafe { CloseHandle(self.0) };
    }
}
