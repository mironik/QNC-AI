//! The QNC mark: white "QNC" in a rounded square, as straight-alpha RGBA pixels of any size
//! (user 2026-11-13: "daj tray-u ikonu QNC"). It is drawn from the font QNC already carries
//! (Hack, bold enough to read at 16 pixels), four times larger, then made smaller, so its edges
//! are smooth. It shows nothing itself: the tray (or anything else) puts the pixels where it needs.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

/// The square behind the letters.
const BACK: [u8; 3] = [0x1f, 0x5f, 0xb4];
/// The letters.
const TEXT: [u8; 3] = [0xff, 0xff, 0xff];
/// Drawn this many times larger, then made smaller.
const OVER: u32 = 4;

/// The mark, `size` x `size` pixels, rows top to bottom, four bytes a pixel (R, G, B, A).
pub fn qnc_mark(size: u32) -> Vec<u8> {
    let size = size.max(8);
    let big = size * OVER;
    let mut cover = vec![0f32; (big * big) as usize];
    let mut ink = vec![0f32; (big * big) as usize];
    let radius = big as f32 * 0.2;
    for y in 0..big {
        for x in 0..big {
            cover[(y * big + x) as usize] = inside_rounded(x as f32 + 0.5, y as f32 + 0.5, big as f32, radius);
        }
    }
    if let Ok(font) = FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR) {
        letters(&font, big, &mut ink);
    }
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut back, mut text) = (0f32, 0f32);
            for dy in 0..OVER {
                for dx in 0..OVER {
                    let at = ((y * OVER + dy) * big + x * OVER + dx) as usize;
                    back += cover[at];
                    text += ink[at].min(cover[at]);
                }
            }
            let samples = (OVER * OVER) as f32;
            let (alpha, text) = (back / samples, text / samples);
            let share = if alpha > 0.0 { (text / alpha).min(1.0) } else { 0.0 };
            for channel in 0..3 {
                let value = f32::from(BACK[channel]) * (1.0 - share) + f32::from(TEXT[channel]) * share;
                pixels.push(value.round() as u8);
            }
            pixels.push((alpha * 255.0).round() as u8);
        }
    }
    pixels
}

/// How much of a pixel at (x, y) is inside the rounded square of `side` with corners of `radius`.
fn inside_rounded(x: f32, y: f32, side: f32, radius: f32) -> f32 {
    let cx = x.clamp(radius, side - radius);
    let cy = y.clamp(radius, side - radius);
    let distance = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (radius - distance + 0.5).clamp(0.0, 1.0)
}

/// "QNC" as large as fits the square with a small margin, in its middle.
fn letters(font: &FontRef<'_>, big: u32, ink: &mut [f32]) {
    let room = big as f32 * 0.86;
    let mut scale = PxScale::from(big as f32);
    let width = |scale: PxScale| -> f32 {
        let scaled = font.as_scaled(scale);
        "QNC".chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum()
    };
    let wide = width(scale);
    if wide > 0.0 {
        scale = PxScale::from(scale.x * room / wide);
    }
    let scaled = font.as_scaled(scale);
    let left = (big as f32 - width(scale)) / 2.0;
    // Capitals stand on the baseline and reach about the ascent minus a little: the middle of
    // their height is put in the middle of the square.
    let cap = scaled.ascent() * 0.72;
    let baseline = (big as f32 + cap) / 2.0;
    let mut x = left;
    for c in "QNC".chars() {
        let id = font.glyph_id(c);
        let glyph = id.with_scale_and_position(scale, ab_glyph::point(x, baseline));
        x += scaled.h_advance(id);
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i64 + i64::from(gx);
                let py = bounds.min.y as i64 + i64::from(gy);
                if px >= 0 && py >= 0 && (px as u32) < big && (py as u32) < big {
                    let at = (py as u32 * big + px as u32) as usize;
                    ink[at] = (ink[at] + coverage).min(1.0);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_has_its_size_clear_corners_and_white_letters_in_the_middle() {
        let size = 32;
        let mark = qnc_mark(size);
        assert_eq!(mark.len(), (size * size * 4) as usize);
        assert_eq!(mark[3], 0, "the corner is clear");
        let white = mark.chunks(4).filter(|p| p[3] == 255 && p[0] > 200 && p[1] > 200 && p[2] > 200).count();
        let back = mark.chunks(4).filter(|p| p[3] == 255 && p[0] < 60).count();
        assert!(white > 20, "letters are drawn: {white}");
        assert!(back > white, "the square shows round them: {back}");
    }
}
