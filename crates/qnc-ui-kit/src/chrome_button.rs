use eframe::egui::{self, Button, Color32, CornerRadius, RichText, Stroke, Ui, Vec2};

/// Colours and height of a chrome button, from the form's layout contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromeButtonStyle {
    pub text: Color32,
    pub border: Color32,
    /// Text and border of a button that is on (one side of a switch).
    pub focus: Color32,
    pub height: f32,
}

/// The flat bordered button of a dock or head row; `on` draws it in the focus colour
/// (the side of a switch that is chosen, e.g. Pokrivalice | Sync/B-roll).
pub fn chrome_button(ui: &mut Ui, text: &str, enabled: bool, on: bool, style: ChromeButtonStyle) -> egui::Response {
    let colour = if on { style.focus } else { style.text };
    let border = if on { style.focus } else { style.border };
    let response = ui.add_enabled(
        enabled,
        Button::new(RichText::new(text).color(colour))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(CornerRadius::same(0))
            .min_size(Vec2::new(0.0, style.height)),
    );
    press_feedback(ui, response)
}

/// The one strong button of a row (e.g. Uvezi): white bold text on the accent fill.
pub fn strong_button(ui: &mut Ui, text: &str, enabled: bool, fill: Color32, height: f32) -> egui::Response {
    let (fill, colour, frame) = on_dark(fill, Color32::WHITE);
    let response = ui.add_enabled(
        enabled,
        Button::new(RichText::new(text).color(colour).strong())
            .fill(fill)
            .stroke(frame.map_or(Stroke::new(0.0, Color32::TRANSPARENT), |frame| Stroke::new(1.0, frame)))
            .corner_radius(CornerRadius::same(0))
            .min_size(Vec2::new(0.0, height)),
    );
    press_feedback(ui, response)
}

/// Fills the rest of a block with its background and gives that rect.
pub fn fill_block(ui: &mut Ui, background: Color32) -> egui::Rect {
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, background);
    rect
}

/// How a button looks, for forms whose buttons differ from the chrome ones (e.g. the
/// Project form: rounded, its UI font size). Everything comes from the form's contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonLook {
    pub text: Color32,
    pub fill: Color32,
    /// A one pixel border, or none.
    pub border: Option<Color32>,
    pub font_size: Option<f32>,
    pub strong: bool,
    /// Square corners (the chrome buttons) or egui's default rounding.
    pub square: bool,
    pub min_size: Vec2,
}

/// A button drawn as `look` says.
pub fn look_button(ui: &mut Ui, text: &str, enabled: bool, mut look: ButtonLook) -> egui::Response {
    let (fill, colour, frame) = on_dark(look.fill, look.text);
    (look.fill, look.text, look.border) = (fill, colour, frame.or(look.border));
    let mut rich = RichText::new(text).color(look.text);
    if let Some(size) = look.font_size {
        rich = rich.size(size);
    }
    if look.strong {
        rich = rich.strong();
    }
    let mut button = Button::new(rich).fill(look.fill).min_size(look.min_size);
    if let Some(border) = look.border {
        button = button.stroke(Stroke::new(1.0, border));
    }
    if look.square {
        button = button.corner_radius(CornerRadius::same(0));
    }
    let response = ui.add_enabled(enabled, button);
    press_feedback(ui, response)
}

/// The small button of a list row (egui's own look, small text), greyed while not `enabled`; `selected` draws it as chosen
/// (e.g. Edit while that segment is edited).
pub fn small_button(ui: &mut Ui, text: &str, enabled: bool, selected: bool) -> egui::Response {
    ui.add_enabled(enabled, Button::new(RichText::new(text).small()).selected(selected))
}

/// A button of egui's own look at a size (e.g. Undo, Save and Close of the trim zoom), put where
/// its caller places it.
pub fn sized_button(text: impl Into<egui::WidgetText>, size: Vec2) -> Button<'static> {
    Button::new(text).min_size(size)
}

/// A button of its caller's text (its size, colour, font), a fill and a 1 px `border` or none,
/// square corners or egui's rounding, at least `size`; put or added where its caller places it.
pub fn framed_button(text: impl Into<egui::WidgetText>, size: Vec2, fill: Color32, border: Option<Color32>, square: bool) -> Button<'static> {
    let mut button = Button::new(text).fill(fill).min_size(size);
    if let Some(border) = border {
        button = button.stroke(Stroke::new(1.0, border));
    }
    if square {
        button = button.corner_radius(CornerRadius::same(0));
    }
    button
}

/// A check box of the gui-kit, greyed while not `enabled`.
pub fn check_box(ui: &mut Ui, checked: &mut bool, text: impl Into<egui::WidgetText>, enabled: bool) -> egui::Response {
    ui.add_enabled(enabled, egui::Checkbox::new(checked, text))
}

/// The key under which a form's theme says whether buttons answer a press (QNC-AI: yes; QNC: no,
/// as before). `qnc-desktop-theme` sets it when it applies the theme.
pub const PRESS_FEEDBACK_KEY: &str = "qnc-press-feedback";

/// How long a button stays lit after a click.
const FLASH: f64 = 0.35;

/// A button that answers (user 2026-11-15: "dugme Dodaj izjavu nema nikakav pokazatelj da je
/// prihvatilo klik"): lighter under the pointer, lighter still while pressed, and lit for a moment
/// after the click, while the work it starts goes on. Only where the theme asks for it.
pub fn press_feedback(ui: &Ui, response: egui::Response) -> egui::Response {
    let on = ui.ctx().data(|data| data.get_temp::<bool>(egui::Id::new(PRESS_FEEDBACK_KEY))).unwrap_or(false);
    if !on || !response.enabled() {
        return response;
    }
    let now = ui.ctx().input(|input| input.time);
    let flash_id = response.id.with("qnc-press-flash");
    if response.clicked() {
        ui.ctx().data_mut(|data| data.insert_temp(flash_id, now));
    }
    let since = ui.ctx().data(|data| data.get_temp::<f64>(flash_id)).map(|at| now - at).filter(|since| *since < FLASH);
    let alpha = if let Some(since) = since {
        ui.ctx().request_repaint();
        (70.0 * (1.0 - since / FLASH)) as u8 + 20
    } else if response.is_pointer_button_down_on() {
        60
    } else if response.hovered() {
        22
    } else {
        0
    };
    if alpha > 0 {
        ui.painter().rect_filled(response.rect, 0.0, Color32::from_white_alpha(alpha));
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A button on the dark desktop (user 2026-11-16: "ova tema je tamna, na njoj ne smije biti
/// bijelih dugmadi"): a light fill is drawn as the dark of its own colour, that colour the text
/// and the frame; a dark or saturated fill (QNC's green) stays as it is with its text. Gives the
/// fill, the text and the frame.
pub fn on_dark(fill: Color32, text: Color32) -> (Color32, Color32, Option<Color32>) {
    let [r, g, b, _] = fill.to_array();
    let light = (0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)) / 255.0;
    if light <= 0.65 {
        return (fill, text, None);
    }
    let dark = |value: u8| (f32::from(value) * 0.16).round() as u8;
    (Color32::from_rgb(dark(r), dark(g), dark(b)), fill, Some(fill))
}

#[cfg(test)]
mod on_dark_tests {
    use super::*;

    #[test]
    fn a_light_fill_turns_dark_with_its_colour_as_text_and_frame_a_dark_or_green_one_stays() {
        let grey = Color32::from_rgb(200, 201, 204);
        let (fill, text, frame) = on_dark(grey, Color32::WHITE);
        assert!(fill.r() < 40 && fill.g() < 40 && fill.b() < 40, "dark: {fill:?}");
        assert_eq!((text, frame), (grey, Some(grey)));
        let green = Color32::from_rgb(16, 185, 129);
        assert_eq!(on_dark(green, Color32::WHITE), (green, Color32::WHITE, None), "QNC's green stays");
    }
}
