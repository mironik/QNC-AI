//! Passive public waveform view component.
//!
//! This component paints prepared peak arrays only. It owns no database,
//! decoder, scanner, probe, player clock or application workflow.

use eframe::egui::{self, Color32, Rect, Stroke};

pub const MODULE_ID: &str = "qnc.module.wave-view";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The level marks of an audio lane (user 2026-10-16, EBU): the alignment level (EBU R 68,
/// -18 dBFS), the permitted maximum (-9 dBFS) and the limit near the top (-1 dBFS).
pub const LEVEL_MARKS_DBFS: [f32; 3] = [-18.0, -9.0, -1.0];

/// Thin lines above and below the middle of a lane at `LEVEL_MARKS_DBFS`, on the scale the
/// peaks are drawn (`gain` as `paint_wave_peaks_gain` uses it); a mark past the lane's
/// edge is not drawn.
pub fn paint_level_marks(painter: &egui::Painter, rect: Rect, gain: f32, colors: [Color32; 3]) {
    let mid = rect.center().y;
    let half = rect.height() * 0.48;
    for (dbfs, color) in LEVEL_MARKS_DBFS.into_iter().zip(colors) {
        let amp = qnc_gain::db_to_gain(f64::from(dbfs)) as f32 * gain;
        if amp > 1.0 {
            continue;
        }
        for y in [mid - amp * half, mid + amp * half] {
            painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], Stroke::new(0.5, color));
        }
    }
}

pub fn paint_wave_peaks(painter: &egui::Painter, rect: Rect, peaks: &[f32], color: Color32) {
    paint_wave_peaks_gain(painter, rect, peaks, 1.0, color);
}

/// The peaks times `gain` (the lane's level, user 2026-10-16): louder than full scale
/// reaches the lane's edge.
pub fn paint_wave_peaks_gain(painter: &egui::Painter, rect: Rect, peaks: &[f32], gain: f32, color: Color32) {
    // Off screen (a Wrap row scrolled away) nothing is built at all.
    if peaks.is_empty() || rect.width() < 2.0 || !painter.clip_rect().intersects(rect) {
        return;
    }
    painter.add(egui::Shape::Mesh(cached_mesh(painter.ctx(), rect, peaks, gain, color)));
}

/// Built meshes, kept while they are painted: a wave changes only with its peaks, its rect,
/// its level or its colour (user 2026-10-20: a level step builds that one lane again).
#[derive(Clone, Default)]
struct WaveMeshes(std::collections::HashMap<u64, (std::sync::Arc<egui::Mesh>, u64)>);

/// Frames a mesh not painted is kept.
const KEEP_PASSES: u64 = 120;

fn cached_mesh(ctx: &egui::Context, rect: Rect, peaks: &[f32], gain: f32, color: Color32) -> std::sync::Arc<egui::Mesh> {
    let key = mesh_key(rect, peaks, gain, color);
    let pass = ctx.cumulative_pass_nr();
    let store = egui::Id::new("qnc-wave-meshes");
    ctx.data_mut(|data| {
        let meshes = data.get_temp_mut_or_default::<WaveMeshes>(store);
        if let Some((mesh, used)) = meshes.0.get_mut(&key) {
            *used = pass;
            return mesh.clone();
        }
        meshes.0.retain(|_, (_, used)| pass.saturating_sub(*used) < KEEP_PASSES);
        let mesh = std::sync::Arc::new(wave_mesh(rect, peaks, gain, color));
        meshes.0.insert(key, (mesh.clone(), pass));
        mesh
    })
}

/// What makes a mesh: the rect, the level, the colour and every peak (a fast fold of their
/// bits; a different wave of the same size is another key).
fn mesh_key(rect: Rect, peaks: &[f32], gain: f32, color: Color32) -> u64 {
    let mut key: u64 = 0xcbf2_9ce4_8422_2325;
    let mut fold = |value: u32| key = (key ^ u64::from(value)).wrapping_mul(0x0000_0100_0000_01b3);
    for value in [rect.min.x, rect.min.y, rect.max.x, rect.max.y, gain] {
        fold(value.to_bits());
    }
    fold(u32::from_le_bytes(color.to_array()));
    fold(peaks.len() as u32);
    peaks.iter().for_each(|peak| fold(peak.to_bits()));
    key
}

/// The wave as one filled shape: the line through the peaks of each pixel column above the
/// middle, its mirror below, the band between them filled (two points a column; user
/// 2026-10-20: cheaper than a column of its own per pixel, and smoother).
fn wave_mesh(rect: Rect, peaks: &[f32], gain: f32, color: Color32) -> egui::Mesh {
    let mid = rect.center().y;
    let half = rect.height() * 0.48;
    let mut mesh = egui::Mesh::default();
    for (x_index, range) in peak_ranges_for_width(peaks.len(), rect.width()).enumerate() {
        let max_peak = peaks[range]
            .iter()
            .map(|peak| peak.abs())
            .fold(0.0_f32, f32::max)
            .mul_add(gain, 0.0)
            .clamp(0.0, 1.0);
        let amp = max_peak * half;
        let x = rect.left() + x_index as f32 + 0.5;
        mesh.colored_vertex(egui::pos2(x, mid - amp), color);
        mesh.colored_vertex(egui::pos2(x, mid + amp), color);
        if x_index > 0 {
            let at = 2 * x_index as u32;
            mesh.add_triangle(at - 2, at - 1, at);
            mesh.add_triangle(at - 1, at + 1, at);
        }
    }
    mesh
}


/// How a wave is drawn: `Mirrored` (QNC: the linear peak above and below the middle) or
/// `Amplitude` (user 2026-11-15, "samo amplitudu kao Premiere i Final Cut": the loudness from the
/// lane's bottom up on a dB scale, `AMPLITUDE_FLOOR_DBFS` at the bottom, full scale at the top, so
/// quiet speech is as readable as loud).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaveShape {
    #[default]
    Mirrored,
    Amplitude,
}

/// The bottom of the amplitude scale.
pub const AMPLITUDE_FLOOR_DBFS: f32 = -54.0;

/// The peaks (linear) from which a column is loud: -9 dBFS (the permitted maximum) and -1 dBFS.
const LOUD_DBFS: [f32; 2] = [0.354_813_4, 0.891_250_9];

/// Where a peak (times its gain) stands in a lane drawn as `shape`: 0 the base, 1 the full height.
pub fn height_of(peak: f32, shape: WaveShape) -> f32 {
    let peak = peak.abs();
    match shape {
        WaveShape::Mirrored => peak.clamp(0.0, 1.0),
        WaveShape::Amplitude => {
            if peak <= 0.0 {
                return 0.0;
            }
            let dbfs = 20.0 * peak.log10();
            ((dbfs - AMPLITUDE_FLOOR_DBFS) / -AMPLITUDE_FLOOR_DBFS).clamp(0.0, 1.0)
        }
    }
}

/// [`paint_wave_peaks_gain`] in the shape the theme asks for.
pub fn paint_wave_shape(painter: &egui::Painter, rect: Rect, peaks: &[f32], gain: f32, color: Color32, shape: WaveShape, loud: [Color32; 2]) {
    if shape == WaveShape::Mirrored {
        return paint_wave_peaks_gain(painter, rect, peaks, gain, color);
    }
    if peaks.is_empty() || rect.width() < 2.0 || !painter.clip_rect().intersects(rect) {
        return;
    }
    painter.add(egui::Shape::Mesh(cached_amplitude(painter.ctx(), rect, peaks, gain, color, loud)));
}

/// [`paint_level_marks`] for a wave drawn as `shape`: the amplitude has no level lines (user
/// 2026-11-15: the louder signal is coloured, one line stays, the level curve), the mirrored wave
/// its marks as before.
pub fn paint_level_marks_shape(painter: &egui::Painter, rect: Rect, gain: f32, colors: [Color32; 3], shape: WaveShape) {
    if shape == WaveShape::Mirrored {
        paint_level_marks(painter, rect, gain, colors);
    }
}

fn cached_amplitude(ctx: &egui::Context, rect: Rect, peaks: &[f32], gain: f32, color: Color32, loud: [Color32; 2]) -> std::sync::Arc<egui::Mesh> {
    let key = mesh_key(rect, peaks, gain, color) ^ 0x5a5a_5a5a ^ u64::from(u32::from_le_bytes(loud[0].to_array())) ^ (u64::from(u32::from_le_bytes(loud[1].to_array())) << 32);
    let pass = ctx.cumulative_pass_nr();
    let store = egui::Id::new("qnc-wave-meshes");
    ctx.data_mut(|data| {
        let meshes = data.get_temp_mut_or_default::<WaveMeshes>(store);
        if let Some((mesh, used)) = meshes.0.get_mut(&key) {
            *used = pass;
            return mesh.clone();
        }
        meshes.0.retain(|_, (_, used)| pass.saturating_sub(*used) < KEEP_PASSES);
        let mesh = std::sync::Arc::new(amplitude_mesh(rect, peaks, gain, color, loud));
        meshes.0.insert(key, (mesh.clone(), pass));
        mesh
    })
}

/// The amplitude as one filled shape from the lane's bottom: two points a pixel column.
fn amplitude_mesh(rect: Rect, peaks: &[f32], gain: f32, color: Color32, loud: [Color32; 2]) -> egui::Mesh {
    let base = rect.bottom();
    let tall = rect.height() * 0.96;
    let mut mesh = egui::Mesh::default();
    for (x_index, range) in peak_ranges_for_width(peaks.len(), rect.width()).enumerate() {
        let peak = peaks[range].iter().map(|peak| peak.abs()).fold(0.0_f32, f32::max) * gain;
        let x = rect.left() + x_index as f32 + 0.5;
        // A column louder than -9 dBFS in the first loud colour, than -1 dBFS in the second (user
        // 2026-11-15: "signal koji je viši bojaj u odgovarajućoj boji", no level lines).
        let shade = if peak >= LOUD_DBFS[1] { loud[1] } else if peak >= LOUD_DBFS[0] { loud[0] } else { color };
        mesh.colored_vertex(egui::pos2(x, base - height_of(peak, WaveShape::Amplitude) * tall), shade);
        mesh.colored_vertex(egui::pos2(x, base), shade);
        if x_index > 0 {
            let at = 2 * x_index as u32;
            mesh.add_triangle(at - 2, at - 1, at);
            mesh.add_triangle(at - 1, at + 1, at);
        }
    }
    mesh
}
pub fn peak_ranges_for_width(
    peak_count: usize,
    width: f32,
) -> impl Iterator<Item = std::ops::Range<usize>> {
    let bars = width.floor().max(1.0) as usize;
    (0..bars).map(move |index| {
        let start = index * peak_count / bars;
        let end = ((index + 1) * peak_count / bars)
            .max(start + 1)
            .min(peak_count);
        start..end
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_peak_array_without_owning_timeline_state() {
        let ranges = peak_ranges_for_width(10, 4.0).collect::<Vec<_>>();

        assert_eq!(ranges, [0..2, 2..5, 5..7, 7..10]);
    }

    #[test]
    fn a_wave_is_one_filled_band_two_points_a_column() {
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(4.0, 10.0));
        let mesh = wave_mesh(rect, &[0.5, 0.0, 1.0, 0.25], 1.0, Color32::WHITE);
        assert_eq!(mesh.vertices.len(), 4 * 2);
        assert_eq!(mesh.indices.len(), 3 * 6, "two triangles between neighbouring columns");
        assert!((mesh.vertices[4].pos.y - (5.0 - 4.8)).abs() < 1e-4, "full scale reaches 48 % of the lane above the middle");
        assert_ne!(mesh_key(rect, &[0.5], 1.0, Color32::WHITE), mesh_key(rect, &[0.5], 2.0, Color32::WHITE), "another level, another mesh");
    }

    #[test]
    fn at_least_one_range_is_returned_for_narrow_tracks() {
        let ranges = peak_ranges_for_width(3, 0.5).collect::<Vec<_>>();

        assert_eq!(ranges, [0..3]);
    }
}

#[cfg(test)]
mod amplitude_tests {
    use super::*;

    #[test]
    fn the_amplitude_stands_on_a_db_scale_from_the_bottom() {
        assert_eq!(height_of(1.0, WaveShape::Amplitude), 1.0);
        assert_eq!(height_of(0.0, WaveShape::Amplitude), 0.0);
        let quiet = height_of(0.01, WaveShape::Amplitude); // -40 dBFS: speech far away
        assert!(quiet > 0.2 && quiet < 0.3, "{quiet}: quiet speech stands a quarter high, not a hair");
        assert!((height_of(0.01, WaveShape::Mirrored) - 0.01).abs() < 1e-6);
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(3.0, 100.0));
        let mesh = amplitude_mesh(rect, &[1.0, 0.0, 0.1], 1.0, Color32::WHITE, [Color32::YELLOW, Color32::RED]);
        assert_eq!(mesh.vertices[0].color, Color32::RED, "past -1 dBFS");
        assert_eq!(mesh.vertices[4].color, Color32::WHITE, "-20 dBFS is the wave's own colour");
        assert!((mesh.vertices[0].pos.y - 4.0).abs() < 1e-3, "full scale reaches 96 % up");
        assert_eq!(mesh.vertices[1].pos.y, 100.0, "every column stands on the bottom");
    }
}
