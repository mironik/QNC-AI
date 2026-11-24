//! How far the letters wav2vec2 laid are from where the sound starts (read only: the active
//! project's letters and the clips' sound on this computer; nothing is written).
//! For every word whose start follows a pause (so the sound's start can be seen in the wave),
//! the first letter's start and the transcript word's start are compared with the onset of the
//! voice in the speech channel, in frames. Usage: measure_letters QNC_ROOT [CLIP_ID]

use std::{path::Path, sync::Arc};

/// Sub-frame steps the level is measured in.
const STEPS_PER_FRAME: usize = 10;
/// The quiet before an onset (seconds).
const QUIET_BEFORE: f64 = 0.06;
/// How far from the letter the onset is looked for (frames).
const SEARCH: f64 = 10.0;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(args.first().ok_or("usage: measure_letters QNC_ROOT [CLIP_ID]")?);
    let only = args.get(1);
    let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
    let snapshot = active.read().map_err(|e| e.to_string())?;
    let reader = active.settings_reader().clone();
    let settings = snapshot.settings;
    println!("project {}", snapshot.project_uri);
    let db = qnc_db_broker::ProjectDbTarget::for_project(&reader, &settings)?;
    let channel = u16::try_from(settings.transcribe_channel().map_err(|e| e.to_string())?).map_err(|_| "no speech channel")?;
    let content = qnc_content_read::ContentReader::for_project(&reader, &settings)?;
    let sources = qnc_source_bindings::load(root)?.sources;
    let target = qnc_content_store::ContentTarget::for_project(&reader, &settings).map_err(|e| e.to_string());
    let clips = Arc::new(qnc_player_clip_content::PlayerClipContent { target });
    let locator = qnc_still_locator::StillLocator::new(reader, settings, sources, clips);
    let (mut letter_diffs, mut word_diffs, mut rows) = (Vec::new(), Vec::new(), Vec::new());
    for clip in content.summaries()? {
        if only.is_some_and(|only| only != &clip.clip_id && only != &clip.name) {
            continue;
        }
        let words = qnc_letter_db::read_clip(&db, &clip.clip_id)?;
        let timed: Vec<(u64, String, f64)> = words
            .iter()
            .filter_map(|word| word.letters.iter().find_map(|letter| letter.from).map(|first| (word.word_from, word.text.clone(), first)))
            .collect();
        if timed.is_empty() {
            continue;
        }
        let place = match locator.locate_sound(&clip.clip_id) {
            Ok(place) => place,
            Err(error) => {
                println!("{}: sound not here: {error}", clip.name);
                continue;
            }
        };
        let fps = f64::from(place.fps.0) / f64::from(place.fps.1.max(1));
        let first = timed.iter().map(|(_, _, from)| *from).fold(f64::MAX, f64::min).max(30.0) as u64 - 30;
        let last = timed.iter().map(|(_, _, from)| *from).fold(0.0, f64::max) as u64 + 30;
        let sound = qnc_sound_range::read(&place, channel, first, last, &mut |_| true)?;
        // The level every tenth of a frame: RMS over 10 ms, in dBFS, on the clip's frame axis.
        let rate = f64::from(sound.rate.max(1));
        let frame_of = |sample: usize| (sound.from_seconds + sample as f64 / rate - place.frame_zero_seconds) * fps;
        let step = (rate / fps / STEPS_PER_FRAME as f64).max(1.0) as usize;
        let window = (rate * 0.01) as usize;
        let mut levels: Vec<(f64, f32)> = Vec::new();
        let mut at = 0;
        while at + window <= sound.samples.len() {
            let power = sound.samples[at..at + window].iter().map(|s| s * s).sum::<f32>() / window as f32;
            levels.push((frame_of(at + window / 2), 10.0 * power.max(1e-12).log10()));
            at += step;
        }
        let mut sorted: Vec<f32> = levels.iter().map(|(_, db)| *db).collect();
        sorted.sort_by(f32::total_cmp);
        let floor = sorted.get(sorted.len() / 5).copied().unwrap_or(-90.0);
        let loud = sorted.get(sorted.len() * 9 / 10).copied().unwrap_or(-20.0);
        // Voice: well over the quiet and no more than 30 dB under the loud part.
        let threshold = (floor + 12.0).max(loud - 30.0);
        let quiet_steps = (QUIET_BEFORE * fps * STEPS_PER_FRAME as f64).round() as usize;
        let onsets: Vec<f64> = (quiet_steps..levels.len())
            .filter(|&i| levels[i].1 > threshold && levels[i - quiet_steps..i].iter().all(|(_, db)| *db <= threshold))
            .map(|i| levels[i].0)
            .collect();
        for (word_from, text, letter) in &timed {
            let Some(onset) = onsets.iter().copied().filter(|onset| (onset - letter).abs() <= SEARCH).min_by(|a, b| (a - letter).abs().total_cmp(&(b - letter).abs())) else { continue };
            letter_diffs.push(letter - onset);
            word_diffs.push(*word_from as f64 - onset);
            rows.push((clip.name.clone(), text.clone(), onset, *letter, *word_from));
        }
        println!("{}: {} words with letters, threshold {threshold:.0} dBFS (quiet {floor:.0}, loud {loud:.0})", clip.name, timed.len());
    }
    let report = |name: &str, diffs: &[f64]| {
        if diffs.is_empty() {
            println!("{name}: nothing measured");
            return;
        }
        let mut sorted = diffs.to_vec();
        sorted.sort_by(f64::total_cmp);
        let mut abs: Vec<f64> = diffs.iter().map(|d| d.abs()).collect();
        abs.sort_by(f64::total_cmp);
        let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
        let within = |frames: f64| 100.0 * abs.iter().filter(|d| **d <= frames).count() as f64 / abs.len() as f64;
        println!(
            "{name}: n={} mean {:+.2} median {:+.2} frames | |diff| p50 {:.2} p90 {:.2} max {:.2} | within 1 frame {:.0}% within 2 {:.0}%",
            diffs.len(), mean, sorted[sorted.len() / 2], abs[abs.len() / 2], abs[abs.len() * 9 / 10], abs[abs.len() - 1], within(1.0), within(2.0)
        );
    };
    println!("(+ is later than the sound starts, - earlier; frames of the clip)");
    report("letters (wav2vec2)", &letter_diffs);
    report("words (transcript)", &word_diffs);
    rows.sort_by(|a, b| (b.3 - b.2).abs().total_cmp(&(a.3 - a.2).abs()));
    println!("largest letter differences:");
    for (clip, text, onset, letter, word) in rows.iter().take(12) {
        println!("  {clip} \"{text}\": sound {onset:.1}, letter {letter:.1} ({:+.1}), word {word} ({:+.1})", letter - onset, *word as f64 - onset);
    }
    Ok(())
}
