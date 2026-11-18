//! Editorial form base for Media Assist groups and Story: the Ingest layout
//! and UI copied 1:1 (shell, preview monitor, pool head, source dock with the
//! timeline), without the right clip grid; the place of the Ingest directory
//! browser shows the clip list. Passive: the host fills `EditorialView` and
//! receives `EditorialIntent`s.

mod app;
use qnc_editorial_layout as layout_contract;
use qnc_editorial_layout::theme;

use eframe::egui;

pub use app::EditorialApp;
pub use layout_contract::{check_contracts_message, EditorialContracts};
pub use qnc_editorial_application::{
    action_ids, locate_qnc_root, EditorialClip, EditorialIntent, EditorialView, MonitorFrame,
};

/// One editorial form for a group (e, g, l or o).
pub struct EditorialForm {
    contracts: EditorialContracts,
    theme: theme::Theme,
}

impl EditorialForm {
    pub fn new(group: &str) -> Result<Self, String> {
        let contracts = EditorialContracts::load(group)?;
        let theme = theme::Theme::from_contract(&contracts.shell);
        Ok(Self { contracts, theme })
    }

    /// The Tab walk of this group's form (its layout contract).
    pub(crate) fn focus_layout(&self) -> qnc_editorial_application::FocusLayout {
        // `--pool-tab <tab action>`: the tab the opener asks for (the tray's Breaking news opens All,
        // there is no transcript yet; user 2026-11-18), else the layout's own.
        let asked = std::env::args().skip_while(|arg| arg != "--pool-tab").nth(1);
        let opening = asked.as_deref().or(self.contracts.opening_tab());
        qnc_editorial_application::FocusLayout::new(self.contracts.focus_names()).with_opening_tab(opening)
    }

    pub(crate) fn shortcuts(&self) -> &qnc_keyboard_shortcut::ShortcutCatalog {
        &self.contracts.shortcuts
    }

    /// Applies fonts and visuals of the contract theme.
    pub fn apply_theme(&self, ctx: &egui::Context) {
        theme::apply(ctx, &self.theme);
    }

    /// Paints the whole form into `ui`; returns the intent of this frame.
    pub fn show_desktop(&self, ui: &mut egui::Ui, frame: &mut qnc_board::Frame<'_>, view: &EditorialView) -> Option<EditorialIntent> {
        qnc_editorial_blocks::render_desktop(ui, frame, &self.contracts, &self.theme, view)
    }
}
