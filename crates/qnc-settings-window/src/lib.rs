//! The window of its own a block of QNC-AI's Settings form opens (user 2026-11-23: "samostalna kocka
//! koja ima svoj GUI koji će se pozivati iz Forme Postavke"; "postavi je na sredinu ekrana";
//! "forma treba imati fiksne dimenzije koje je moguće editirati preko skrivene točke"; "treba biti
//! stilski isto"). One piece for every such window (the export templates, the AI settings), so it
//! is written once:
//!
//! - in the middle of the screen, at a fixed size;
//! - its own bar, whose hidden dot opens its own look editor (`contracts/ui/forms/<form>.look.json`);
//! - a menu on the left, a panel on the right, their widths, the margin and the grid of the fields
//!   (the Settings form's Advanced grid: the label above its field) from that look.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use eframe::egui;
use qnc_desktop_theme::Theme;
use qnc_settings_rows::PanelMetrics;

/// The measures every such window's look has.
pub const LOOK_KEYS: [&str; 9] = ["window_width", "window_height", "list_width", "right_width", "margin", "label_size", "min_cell_width", "gap", "cell_height"];

/// Which side the window's body draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// The menu on the left.
    Menu,
    /// The panel on the right.
    Panel,
}

/// How the window's fields look: the grid of the fields and the rows of the panel.
pub struct Style {
    pub grid: qnc_settings_form::GridStyle,
    pub metrics: PanelMetrics,
}

thread_local! {
    /// The bar and look of each window open, by its look's form name.
    static OPEN: RefCell<HashMap<String, (qnc_title_bar::TitleBar, qnc_form_look::FormLook)>> = RefCell::new(HashMap::new());
}

/// Opens the window whose look is `form` (`title` on its bar); a look that cannot be read is an error.
pub fn open(root: &Path, form: &str, title: &str) -> Result<(), String> {
    let look = qnc_form_look::FormLook::load(root, form, &LOOK_KEYS)?;
    let bar = qnc_title_bar::TitleBar::new(title, root.to_path_buf()).with_look_of(form);
    OPEN.with(|open| open.borrow_mut().insert(form.to_string(), (bar, look)));
    Ok(())
}

/// Whether the window `form` is open.
pub fn is_open(form: &str) -> bool {
    OPEN.with(|open| open.borrow().contains_key(form))
}

/// Draws the window `form` while it is open: `body` draws the menu, then the panel. Gives whether it
/// is still open (its close button or the system closed it).
pub fn show(ctx: &egui::Context, root: &Path, form: &str, title: &str, metrics: &PanelMetrics, theme: &Theme, mut body: impl FnMut(&mut egui::Ui, Pane, &Style)) -> bool {
    let mut still = true;
    OPEN.with(|cell| {
        let mut cell = cell.borrow_mut();
        let Some((bar, look)) = cell.get_mut(form) else {
            still = false;
            return;
        };
        let size = egui::vec2(look.get("window_width"), look.get("window_height") + qnc_title_bar::HEIGHT);
        let screen = ctx.input(|input| input.viewport().monitor_size).unwrap_or(size);
        let place = ((screen - size) / 2.0).max(egui::Vec2::ZERO);
        let builder = qnc_title_bar::frameless(egui::ViewportBuilder::default().with_title(title).with_inner_size(size).with_position(egui::pos2(place.x, place.y)).with_resizable(false));
        let viewport = egui::ViewportId::from_hash_of(("qnc-settings-window", form, root.to_string_lossy().to_string()));
        ctx.show_viewport_immediate(viewport, builder, |ctx, _| {
            if ctx.input(|input| input.viewport().close_requested()) {
                still = false;
            }
            bar.show(ctx);
            look.editor_of_own(ctx);
            look.window(ctx, qnc_title_bar::HEIGHT);
            let style = Style {
                grid: qnc_settings_form::GridStyle {
                    label_color: theme.text_muted,
                    label_size: look.get("label_size"),
                    min_cell_width: look.get("min_cell_width"),
                    gap: look.get("gap"),
                    cell_height: look.get("cell_height"),
                    label_beside: None,
                },
                metrics: PanelMetrics { label_font_size: look.get("label_size"), group_title_font_size: look.get("label_size"), ..metrics.clone() },
            };
            let margin = look.get("margin").round() as i8;
            let (list_width, right_width) = (look.get("list_width"), look.get("right_width"));
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg).inner_margin(egui::Margin::same(margin))).show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(list_width);
                        body(ui, Pane::Menu, &style);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(right_width);
                        body(ui, Pane::Panel, &style);
                    });
                });
            });
        });
        if !still {
            cell.remove(form);
        }
    });
    still
}
