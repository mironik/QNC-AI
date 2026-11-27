//! The look of every piece from the one desktop theme (user 2026-10-20, rule 11: the Ingest,
//! Story and Project forms and their layout models each turned the theme into the same
//! styles). One function per piece; it knows no form.

use eframe::egui::Color32;
use qnc_desktop_theme::Theme;

pub const MODULE_ID: &str = "qnc.module.theme-styles";

/// The colours of every timeline (and of its trim zoom, the timeline enlarged); the gui-kit holds
/// them (C15), the timeline only draws with them.
pub fn timeline(theme: &Theme) -> qnc_timeline::TimelineTheme {
    let media = &theme.media;
    let c = qnc_desktop_theme::rgb;
    let rgb = |r, g, b| Color32::from_rgb(r, g, b);
    qnc_timeline::TimelineTheme {
        background: theme.bg,
        label_background: theme.surface_alt,
        video_background: theme.surface,
        // Audio lanes, waves, focus, meter: the media colours of the theme (QNC: a story's green
        // segment, user 2026-10-14; QNC-AI graphite, user 2026-11-15).
        audio_primary_background: c(media.lane),
        audio_secondary_background: c(media.lane),
        border: theme.border_soft,
        text: theme.text,
        muted: theme.text_muted,
        playhead: theme.accent,
        focus: c(media.timeline_focus),
        shot_range: rgb(96, 165, 250),
        in_out_dim: Color32::from_black_alpha(133),
        used: Color32::from_rgba_unmultiplied(170, 170, 170, 46),
        wave_a1: c(media.wave),
        wave_a2: c(media.wave),
        wave_a3: c(media.wave),
        wave_a4: c(media.wave),
        audio_wave: c(media.wave_zoom),
        lane_taken: c(media.lane_taken),
        lane_silent: rgb(200, 45, 45),
        marker_hover: rgb(230, 40, 40),
        level_marks: media.level_marks.map(|[r, g, b, a]| Color32::from_rgba_unmultiplied(r, g, b, a)),
        level_curve: c(media.level_curve),
        cut: rgb(230, 30, 30),
        cut_side: rgb(255, 64, 64),
        unheard: Color32::from_rgba_premultiplied(70, 10, 10, 70),
        meter: media.meter.map(c),
        lane_divider: media.lane_divider.map_or(Color32::TRANSPARENT, c),
        lane_quiet: media.lane_quiet,
        wave_shape: if media.wave_amplitude { qnc_timeline::WaveShape::Amplitude } else { qnc_timeline::WaveShape::Mirrored },
    }
}

/// The chrome of a dock and its header; `header_timeline_gap` is the form's measure.
pub fn dock(theme: &Theme, header_timeline_gap: f32) -> qnc_source_dock::TimelineDockStyle {
    qnc_source_dock::TimelineDockStyle {
        fill: theme.panel_alt,
        border: theme.border,
        text: theme.text,
        font_ui: theme.font_ui,
        chrome_row_height: theme.chrome_row_height,
        chrome_control_height: theme.chrome_control_height,
        chrome_pad_x: theme.chrome_pad_x,
        chrome_pad_y: theme.chrome_pad_y,
        header_timeline_gap,
    }
}

/// A row of tabs and commands; `tab_gap` is the form's measure.
pub fn row(theme: &Theme, tab_gap: f32) -> qnc_media_pool_head::RowStyle {
    qnc_media_pool_head::RowStyle {
        text: theme.text,
        muted: theme.text_muted,
        accent: theme.accent,
        border: theme.border,
        font_ui: theme.font_ui,
        control_height: theme.chrome_control_height,
        tab_gap,
    }
}

/// A chrome button.
pub fn button(theme: &Theme) -> qnc_ui_kit::ChromeButtonStyle {
    qnc_ui_kit::ChromeButtonStyle { text: theme.text, border: theme.border, focus: theme.focus, height: theme.chrome_control_height }
}

/// A bar of confirm and cancel buttons.
pub fn action_bar(theme: &Theme) -> qnc_ui_kit::FormActionBarStyle {
    qnc_ui_kit::FormActionBarStyle::new(theme.text, theme.accent, theme.border, theme.font_ui, theme.chrome_control_height)
}

/// The preview monitor.
pub fn monitor(theme: &Theme) -> qnc_monitor::MonitorChrome {
    qnc_monitor::MonitorChrome { fill: theme.black, border: theme.border, muted: theme.text_muted, font_size: theme.font_ui }
}

/// The media pool cards.
pub fn cards(theme: &Theme) -> qnc_media_card::CardStyle {
    qnc_media_card::CardStyle {
        raised: theme.surface_alt,
        surface: theme.surface,
        border: theme.border,
        text: theme.text,
        muted: theme.text_muted,
        select_red: theme.danger,
        dot_ready: qnc_desktop_theme::rgb(theme.media.dot_ready),
        dot_original_ready: qnc_desktop_theme::rgb(theme.media.dot_original_ready),
        dot_pending: qnc_desktop_theme::rgb(theme.media.dot_pending),
        dot_error: qnc_desktop_theme::rgb(theme.media.dot_error),
        check_on: qnc_desktop_theme::rgb(theme.media.check_on),
        check_mark: Color32::from_rgb(0x1a, 0x1a, 0x1a),
        check_off: Color32::from_rgba_unmultiplied(0, 0, 0, 90),
        check_off_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 140),
        remove_fill: Color32::from_rgba_unmultiplied(0, 0, 0, 120),
        remove_hover: theme.danger.gamma_multiply(0.85),
        remove_mark: Color32::from_rgba_unmultiplied(255, 255, 255, 220),
    }
}

/// The shell palette of a theme of the footer's picker: Dark is the shell contract's, Soft and
/// High contrast are the gui-kit's (gui-kit C15; AGENTS 16: until they are in the UI contract).
pub fn shell_palette(theme: qnc_shell_footer::ThemeId, contract: qnc_shell_footer::Palette) -> qnc_shell_footer::Palette {
    use qnc_shell_footer::{Palette, ThemeId};
    let rgb = |r, g, b| Color32::from_rgb(r, g, b);
    match theme {
        ThemeId::Dark => contract,
        ThemeId::Soft => Palette {
            bg: rgb(22, 27, 38),
            surface: rgb(32, 40, 56),
            raised: rgb(45, 55, 74),
            border: rgb(75, 88, 110),
            text: rgb(236, 239, 244),
            muted: rgb(168, 178, 194),
            accent: rgb(52, 199, 148),
            focus: rgb(255, 196, 90),
        },
        ThemeId::HighContrast => Palette {
            bg: Color32::BLACK,
            surface: rgb(18, 18, 18),
            raised: rgb(36, 36, 36),
            border: rgb(180, 180, 180),
            text: Color32::WHITE,
            muted: rgb(200, 200, 200),
            accent: rgb(0, 255, 170),
            focus: rgb(255, 200, 0),
        },
    }
}
