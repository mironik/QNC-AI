//! The tray menu in QNC-AI's look (user 2026-11-15: "predi i tray u premium stilu, povećaj font"):
//! the menu's items are drawn by the tray itself (owner-drawn), on the clean graphite surface of
//! QNC-AI's shell contract, in a larger font, the item under the pointer raised, an item that is
//! not a choice (the tray's status) muted.

use std::cell::RefCell;

use windows_sys::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetDC, GetDeviceCaps, GetStockObject, GetTextExtentPoint32W, LineTo, MoveToEx, Rectangle, ReleaseDC, SelectObject,
    SetBkMode, SetTextColor, CLEARTYPE_QUALITY, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_NORMAL, HDC, HFONT, LOGPIXELSY, NULL_BRUSH, PS_SOLID, TRANSPARENT,
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
/// The side of a tick box, in points.
const BOX_PT: i32 = 11;
/// The tick in the box: the colour of a chosen thing in QNC-AI.
const TICK: (u8, u8, u8) = (52, 199, 89);

fn rgb((r, g, b): (u8, u8, u8)) -> COLORREF {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
}

thread_local! {
    /// The texts of the menu being shown, by the item data the menu carries (their index).
    static TEXTS: RefCell<Vec<(Vec<u16>, Option<bool>)>> = const { RefCell::new(Vec::new()) };
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

/// The texts of the items about to be shown, each with its box (`MenuItem::check`); item `n` carries
/// `n` as its data.
pub(crate) fn hold(texts: Vec<(Vec<u16>, Option<bool>)>) {
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
    let (text, check) = TEXTS.with(|held| held.borrow().get(item.itemData).cloned()).unwrap_or_default();
    let screen = GetDC(std::ptr::null_mut());
    let font = font();
    let old = SelectObject(screen, font);
    let mut size = SIZE { cx: 0, cy: 0 };
    GetTextExtentPoint32W(screen, text.as_ptr(), text.len() as i32, &mut size);
    SelectObject(screen, old);
    DeleteObject(font);
    ReleaseDC(std::ptr::null_mut(), screen);
    let boxed = if check.is_some() { px(BOX_PT) + px(PAD_X_PT) / 2 } else { 0 };
    item.itemWidth = (size.cx + 2 * px(PAD_X_PT) + boxed) as u32;
    item.itemHeight = px(ROW_PT) as u32;
}

/// WM_DRAWITEM: the row's ground (raised under the pointer), its text (muted when not a choice).
pub(crate) unsafe fn draw(item: *const DRAWITEMSTRUCT) {
    let item = &*item;
    let (text, check) = TEXTS.with(|held| held.borrow().get(item.itemData).cloned()).unwrap_or_default();
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
    let mut left = item.rcItem.left + px(PAD_X_PT);
    if let Some(ticked) = check {
        // The box: a frame in the muted colour, the tick in the chosen colour when ticked.
        let side = px(BOX_PT);
        let top = item.rcItem.top + (item.rcItem.bottom - item.rcItem.top - side) / 2;
        let frame = RECT { left, top, right: left + side, bottom: top + side };
        let pen = CreatePen(PS_SOLID, (side / 9).max(1), rgb(MUTED));
        let none = GetStockObject(NULL_BRUSH);
        let (old_pen, old_brush) = (SelectObject(dc, pen), SelectObject(dc, none));
        Rectangle(dc, frame.left, frame.top, frame.right, frame.bottom);
        SelectObject(dc, old_pen);
        SelectObject(dc, old_brush);
        DeleteObject(pen);
        if ticked {
            let pen = CreatePen(PS_SOLID, (side / 6).max(2), rgb(TICK));
            let old = SelectObject(dc, pen);
            MoveToEx(dc, left + side / 5, top + side / 2, std::ptr::null_mut());
            LineTo(dc, left + side * 2 / 5, top + side * 3 / 4);
            LineTo(dc, left + side * 4 / 5, top + side / 4);
            SelectObject(dc, old);
            DeleteObject(pen);
        }
        left += side + px(PAD_X_PT) / 2;
    }
    let mut place = RECT { left, ..item.rcItem };
    DrawTextW(dc, text.as_ptr(), text.len() as i32, &mut place, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    SelectObject(dc, old);
    DeleteObject(font);
}
