//! An icon in the system tray: its tooltip, its menu (what was chosen comes back on a channel)
//! and a notice beside it; and one running copy of a program. Neutral for its users; the system
//! does it through an adapter: Windows (a hidden message window on its own thread, the shell's
//! notify icon). Another system says it has no tray yet; nothing is made up.

use std::sync::mpsc::Receiver;

#[cfg(windows)]
mod alert;
#[cfg(windows)]
mod windows;

/// What the user did with the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// A menu item, by the id its caller gave it (a click with either button shows the menu).
    Menu(u32),
}

/// One item of the icon's menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub id: u32,
    pub text: String,
    /// A line that only tells (where the story is): grey, nothing happens on it.
    pub enabled: bool,
}

/// The icon in the tray; it goes away when this is dropped.
pub struct Tray {
    #[cfg(windows)]
    inner: windows::Tray,
    events: Receiver<TrayEvent>,
}

impl Tray {
    /// Puts the icon in the tray with its tooltip and menu.
    pub fn start(tip: &str, menu: Vec<MenuItem>) -> Result<Self, String> {
        #[cfg(windows)]
        {
            let (inner, events) = windows::Tray::start(tip, menu)?;
            Ok(Self { inner, events })
        }
        #[cfg(not(windows))]
        {
            let _ = (tip, menu);
            Err("the tray is not made for this system yet".into())
        }
    }

    /// What the user did, when they did something (never waits).
    pub fn try_event(&self) -> Option<TrayEvent> {
        self.events.try_recv().ok()
    }

    /// A notice beside the icon.
    pub fn notify(&self, title: &str, text: &str) {
        #[cfg(windows)]
        self.inner.notify(title, text);
        #[cfg(not(windows))]
        let _ = (title, text);
    }

    /// An error the user must see now: an overlay in the middle of the screen, above every window,
    /// with a button (`ok` is its text) that closes it. It does not wait for the answer.
    pub fn alert(&self, title: &str, text: &str, ok: &str) {
        #[cfg(windows)]
        windows::alert(title, text, ok);
        #[cfg(not(windows))]
        let _ = (title, text, ok);
    }

    /// A new menu.
    pub fn set_menu(&self, menu: Vec<MenuItem>) {
        #[cfg(windows)]
        self.inner.set_menu(menu);
        #[cfg(not(windows))]
        let _ = menu;
    }

    /// A new tooltip.
    pub fn set_tip(&self, tip: &str) {
        #[cfg(windows)]
        self.inner.set_tip(tip);
        #[cfg(not(windows))]
        let _ = tip;
    }
}

/// One running copy of a program on this computer for this user: `None` when another copy runs.
/// The copy holds it while it runs.
pub struct OneCopy {
    #[cfg(windows)]
    _inner: windows::OneCopy,
}

impl OneCopy {
    pub fn take(name: &str) -> Result<Option<Self>, String> {
        #[cfg(windows)]
        {
            Ok(windows::OneCopy::take(name)?.map(|inner| Self { _inner: inner }))
        }
        #[cfg(not(windows))]
        {
            let _ = name;
            Err("one copy of a program is not made for this system yet".into())
        }
    }
}
