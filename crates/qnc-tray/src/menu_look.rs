//! The tray menu in QNC-AI's look (user 2026-11-15: "predi i tray u premium stilu, povećaj font"):
//! the menu's items are drawn by the tray itself (owner-drawn), on the clean graphite surface of
//! QNC-AI's shell contract, in a larger font, the item under the pointer raised, an item that is
//! not a choice (the tray's status) muted.

use std::cell::RefCell;

use windows_sys::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetDC, GetDeviceCaps, GetTextExtentPoint32W, ReleaseDC, SelectObject,
    SetBkMode, SetTextColor, CLEARTYPE_QUALITY, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_NORMAL, HDC, HFONT, LOGPIXELSY, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_GRAYED, ODS_SELECTED};
use windows_sys::Win32::UI::WindowsAndMessaging::{SetMenuInfo, HMENU, MENUINFO, MIM_BACKGROUND};

/// The clean palette of QNC-AI (`contracts/ui/shell.layout.json`): surface, raised, text, muted.
const SURFACE: (u8, u8, u8) = (23, 24, 27);
const RAISED: (u8, u8, u8) = (38, 40, 44);
const TEXT: (u8, u8, u8) = (237, 237, 237);
const MUTED: (u8, u8, u8) = (138, 139, 143);
/// The font height in points and the room round an item's text, in points too.
const FONT_PT: i32 = 12;
const PAD_X_PT: i32 = 14;
const ROW_PT: i32 = 26;

fn rgb((r, g, b): (u8, u8, u8)) -> COLORREF {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
}

thread_local! {
    /// The texts of the menu being shown, by the item data the menu carries (their index).
    static TEXTS: RefCell<Vec<Vec<u16>>> = const { RefCell::new(Vec::new()) };
}

/// Points as pixels of the screen.
unsafe fn px(points: i32) -> i32 {
    let screen = GetDC(std::ptr::null_mut());
    let dpi = GetDeviceCaps(screen, LOGPIXELSY as i32);
    ReleaseDC(std::ptr::null_mut(), screen);
    points * dpi / 72
}

unsafe fn font() -> HFONT {
    let face: Vec<u16> = "Segoe UI".encode_utf16().chain([0]).collect();
    CreateFontW(-px(FONT_PT), 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, 0, 0, 0, u32::from(CLEARTYPE_QUALITY), 0, face.as_ptr())
}

/// The texts of the items about to be shown; item `n` carries `n` as its data.
pub(crate) fn hold(texts: Vec<Vec<u16>>) {
    TEXTS.with(|held| *held.borrow_mut() = texts);
}

/// The menu's own ground in the surface colour (round the items).
pub(crate) unsafe fn ground(menu: HMENU) -> isize {
    let brush = CreateSolidBrush(rgb(SURFACE));
    let info = MENUINFO { cbSize: std::mem::size_of::<MENUINFO>() as u32, fMask: MIM_BACKGROUND, dwStyle: 0, cyMax: 0, hbrBack: brush, dwContextHelpID: 0, dwMenuData: 0 };
    SetMenuInfo(menu, &info);
    brush as isize
}

pub(crate) unsafe fn forget_ground(brush: isize) {
    DeleteObject(brush as _);
}

/// WM_MEASUREITEM: the text's width in the menu font and the room round it.
pub(crate) unsafe fn measure(item: *mut MEASUREITEMSTRUCT) {
    let item = &mut *item;
    let text = TEXTS.with(|held| held.borrow().get(item.itemData).cloned()).unwrap_or_default();
    let screen = GetDC(std::ptr::null_mut());
    let font = font();
    let old = SelectObject(screen, font);
    let mut size = SIZE { cx: 0, cy: 0 };
    GetTextExtentPoint32W(screen, text.as_ptr(), text.len() as i32, &mut size);
    SelectObject(screen, old);
    DeleteObject(font);
    ReleaseDC(std::ptr::null_mut(), screen);
    item.itemWidth = (size.cx + 2 * px(PAD_X_PT)) as u32;
    item.itemHeight = px(ROW_PT) as u32;
}

/// WM_DRAWITEM: the row's ground (raised under the pointer), its text (muted when not a choice).
pub(crate) unsafe fn draw(item: *const DRAWITEMSTRUCT) {
    let item = &*item;
    let text = TEXTS.with(|held| held.borrow().get(item.itemData).cloned()).unwrap_or_default();
    let grey = item.itemState & ODS_GRAYED != 0;
    let lit = item.itemState & ODS_SELECTED != 0 && !grey;
    let dc: HDC = item.hDC;
    let brush = CreateSolidBrush(rgb(if lit { RAISED } else { SURFACE }));
    FillRect(dc, &item.rcItem, brush);
    DeleteObject(brush);
    let font = font();
    let old = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, rgb(if grey { MUTED } else { TEXT }));
    let mut place = RECT { left: item.rcItem.left + px(PAD_X_PT), ..item.rcItem };
    DrawTextW(dc, text.as_ptr(), text.len() as i32, &mut place, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    SelectObject(dc, old);
    DeleteObject(font);
}
