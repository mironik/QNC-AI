//! The title bar of every QNC-AI window (user 2026-11-15: "pretragu bi dodao kao samostalnu kocku
//! koja bi se pozivala u obliku overlay prozora, a link bi stavio u hederu prozora pored dugmadi
//! close, maximize, minimize; tu bi još dodao i ikonu postavki (zupčanik)"; chosen: QNC's own title
//! bar, in every QNC-AI window, the gear opens the existing Settings).
//!
//! A system title bar takes no buttons of an application, so the window is made without the system
//! frame ([`frameless`]) and this block draws the bar: the title on the left; on the right the
//! archive search (`qnc-base-search-view`, an overlay over the window), the gear (QNC-AI Settings),
//! minimize, maximize and close. Dragging the bar moves the window, a double click maximizes it,
//! its edges resize it. The same on Windows, macOS and Linux.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, ResizeDirection, Sense, Stroke, Vec2, ViewportCommand};
use qnc_base_search_view::SearchOverlay;
use qnc_wizard_text::text;

/// The height of the bar.
pub const HEIGHT: f32 = 34.0;
/// The width of each button of the bar.
const BUTTON: f32 = 44.0;
/// How far inside the window's edge a press resizes it.
const EDGE: f32 = 6.0;

/// The viewport of a window without the system frame, for this bar.
pub fn frameless(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    viewport.with_decorations(false).with_resizable(true)
}

/// What the bar was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Search,
    Settings,
    Minimize,
    Maximize,
    Close,
}

/// The bar of one window.
pub struct TitleBar {
    title: String,
    root: PathBuf,
    search: SearchOverlay,
}

impl TitleBar {
    pub fn new(title: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self { title: title.into(), root: root.into(), search: SearchOverlay::default() }
    }

    /// Draws the bar on top of the window (before its content) and does what was asked; then the
    /// archive search over the window when it is open.
    pub fn show(&mut self, ctx: &egui::Context) {
        let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
        if !maximized {
            resize_edges(ctx);
        }
        let mut pressed = None;
        egui::TopBottomPanel::top("qnc-title-bar")
            .exact_height(HEIGHT)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.panel_fill))
            .show_separator_line(true)
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                let drag = ui.interact(bar, egui::Id::new("qnc-title-bar-drag"), Sense::click_and_drag());
                if drag.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                let colour = ui.visuals().text_color();
                ui.painter().text(Pos2::new(bar.left() + 14.0, bar.center().y), egui::Align2::LEFT_CENTER, &self.title, egui::FontId::proportional(14.0), colour);
                let buttons = [
                    (Button::Close, "title.close"),
                    (Button::Maximize, "title.maximize"),
                    (Button::Minimize, "title.minimize"),
                    (Button::Settings, "title.settings"),
                    (Button::Search, "title.search"),
                ];
                for (at, (button, tip)) in buttons.into_iter().enumerate() {
                    let rect = Rect::from_min_size(Pos2::new(bar.right() - BUTTON * (at as f32 + 1.0), bar.top()), Vec2::new(BUTTON, bar.height()));
                    let response = ui.interact(rect, egui::Id::new(("qnc-title-bar", at)), Sense::click()).on_hover_text(text(tip));
                    let hovered = response.hovered();
                    if hovered {
                        let fill = if button == Button::Close { Color32::from_rgb(196, 43, 28) } else { ui.visuals().widgets.hovered.weak_bg_fill };
                        ui.painter().rect_filled(rect, 0.0, fill);
                    }
                    let stroke = Stroke::new(1.3, if hovered && button == Button::Close { Color32::WHITE } else { colour });
                    paint_icon(ui.painter(), rect.center(), button, maximized, stroke);
                    if response.clicked() {
                        pressed = Some(button);
                    }
                }
            });
        match pressed {
            Some(Button::Close) => ctx.send_viewport_cmd(ViewportCommand::Close),
            Some(Button::Maximize) => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
            Some(Button::Minimize) => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            Some(Button::Search) => self.search.open(),
            Some(Button::Settings) => open_settings(&self.root),
            None => {}
        }
        self.search.show(ctx, &self.root);
    }
}

/// QNC-AI Settings, the program beside this one.
fn open_settings(root: &Path) {
    let line = match qnc_ai_open::open(root, "qnc-ai-settings") {
        Ok(program) => format!("title-bar settings opened {}", program.display()),
        Err(error) => format!("title-bar settings not opened: {error}"),
    };
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

/// The icon of a button, drawn in lines (the same on every system and font).
fn paint_icon(painter: &egui::Painter, centre: Pos2, button: Button, maximized: bool, stroke: Stroke) {
    let at = |x: f32, y: f32| centre + Vec2::new(x, y);
    match button {
        Button::Close => {
            painter.line_segment([at(-5.0, -5.0), at(5.0, 5.0)], stroke);
            painter.line_segment([at(-5.0, 5.0), at(5.0, -5.0)], stroke);
        }
        Button::Maximize if maximized => {
            painter.rect_stroke(Rect::from_min_max(at(-5.0, -3.0), at(3.0, 5.0)), 0.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([at(-3.0, -5.0), at(5.0, -5.0)], stroke);
            painter.line_segment([at(5.0, -5.0), at(5.0, 3.0)], stroke);
        }
        Button::Maximize => {
            painter.rect_stroke(Rect::from_min_max(at(-5.0, -5.0), at(5.0, 5.0)), 0.0, stroke, egui::StrokeKind::Middle);
        }
        Button::Minimize => {
            painter.line_segment([at(-5.0, 0.0), at(5.0, 0.0)], stroke);
        }
        Button::Search => {
            painter.circle_stroke(at(-1.5, -1.5), 4.5, stroke);
            painter.line_segment([at(1.8, 1.8), at(6.0, 6.0)], stroke);
        }
        Button::Settings => {
            painter.circle_stroke(centre, 2.5, stroke);
            painter.circle_stroke(centre, 5.5, stroke);
            for tooth in 0..8 {
                let angle = tooth as f32 * std::f32::consts::FRAC_PI_4;
                let (sin, cos) = angle.sin_cos();
                painter.line_segment([at(cos * 5.5, sin * 5.5), at(cos * 7.5, sin * 7.5)], stroke);
            }
        }
    }
}

/// The edges of a window without the system frame resize it, as a system window's do.
fn resize_edges(ctx: &egui::Context) {
    let Some(pointer) = ctx.input(|input| input.pointer.hover_pos()) else { return };
    let screen = ctx.screen_rect();
    let (left, right) = (pointer.x <= screen.left() + EDGE, pointer.x >= screen.right() - EDGE);
    let (top, bottom) = (pointer.y <= screen.top() + EDGE, pointer.y >= screen.bottom() - EDGE);
    let direction = match (left, right, top, bottom) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::West,
        (_, true, ..) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        (.., true) => ResizeDirection::South,
        _ => return,
    };
    ctx.set_cursor_icon(match direction {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
    });
    if ctx.input(|input| input.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

/// An app in a window with this bar: the bar first, then the app below it.
pub struct Framed<A> {
    pub bar: TitleBar,
    pub app: A,
}

impl<A: eframe::App> eframe::App for Framed<A> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.bar.show(ctx);
        self.app.update(ctx, frame);
    }
}
