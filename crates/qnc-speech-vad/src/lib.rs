//! Where speech is in a sound (user 2026-11-25: Silero VAD before the transcription, in place of
//! the loudness floor). Silero VAD v5 (MIT) as its ONNX file, run through the ONNX Runtime
//! library the caller names (the one beside the program), on one thread of the processor, inside
//! the program that asks: some three hundred times faster than real time (probe 2026-11-25: 157 s
//! of sound in 0.5 s), so a plug-in process would cost more than it. It is given 16 kHz mono
//! samples and gives the spans of speech; it reads the model and writes nothing.
//!
//! What it finds is a human voice: a voice in the background or children counts as speech too
//! (probe 2026-11-25), silence, music under nobody and room noise do not.

use std::ops::Range;
use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;

pub const MODULE_ID: &str = "qnc.module.speech-vad";

/// The rate the model hears.
pub const RATE: usize = 16_000;
/// One step: 512 samples (32 ms), with 64 of context before it (Silero v5).
const CHUNK: usize = 512;
const CONTEXT: usize = 64;
/// Speech starts at this probability and ends under the lower one held for `QUIET_SECONDS`
/// (Silero's own settings); a piece shorter than `LEAST_SECONDS` is not speech.
const ON: f32 = 0.5;
const OFF: f32 = 0.35;
const QUIET_SECONDS: f64 = 0.3;
const LEAST_SECONDS: f64 = 0.25;
/// Room kept before and after each piece, so the engine hears a word's first and last sound.
const PAD_SECONDS: f64 = 0.2;
/// Pieces closer than this are one (a breath between two sentences stays with them).
const JOIN_SECONDS: f64 = 1.0;

/// A loaded model, kept for the next sound.
pub struct SpeechVad {
    session: Session,
}

impl SpeechVad {
    /// Loads the model `model` (`silero_vad.onnx`) with the ONNX Runtime library `runtime`.
    pub fn load(model: &Path, runtime: &Path) -> Result<Self, String> {
        ort::init_from(runtime.to_string_lossy()).commit().map_err(err)?;
        let session = Session::builder().and_then(|builder| builder.with_intra_threads(1)).and_then(|builder| builder.commit_from_file(model)).map_err(err)?;
        Ok(Self { session })
    }

    /// The spans of speech in `samples` (16 kHz mono), padded and joined, in samples.
    pub fn speech(&mut self, samples: &[f32]) -> Result<Vec<Range<usize>>, String> {
        Ok(spans(&self.probabilities(samples)?, samples.len()))
    }

    /// The speech probability of every 32 ms.
    fn probabilities(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        let mut state = vec![0f32; 2 * 128];
        let mut context = vec![0f32; CONTEXT];
        let mut out = Vec::with_capacity(samples.len() / CHUNK + 1);
        for chunk in samples.chunks(CHUNK) {
            let mut input = context.clone();
            input.extend_from_slice(chunk);
            input.resize(CONTEXT + CHUNK, 0.0);
            let result = self
                .session
                .run(ort::inputs![
                    "input" => Tensor::from_array(([1usize, CONTEXT + CHUNK], input.clone())).map_err(err)?,
                    "state" => Tensor::from_array(([2usize, 1, 128], state)).map_err(err)?,
                    "sr" => Tensor::from_array(([1usize], vec![RATE as i64])).map_err(err)?
                ])
                .map_err(err)?;
            out.push(result["output"].try_extract_tensor::<f32>().map_err(err)?.1[0]);
            state = result["stateN"].try_extract_tensor::<f32>().map_err(err)?.1.to_vec();
            context = input[input.len() - CONTEXT..].to_vec();
        }
        Ok(out)
    }
}

/// The speech pieces of the probabilities (one for every `CHUNK` samples) of a sound of `length`
/// samples: on at `ON`, off under `OFF` held `QUIET_SECONDS`, shorter than `LEAST_SECONDS` left
/// out, then padded by `PAD_SECONDS` and joined over gaps under `JOIN_SECONDS`.
pub fn spans(probabilities: &[f32], length: usize) -> Vec<Range<usize>> {
    let seconds = |chunks: usize| chunks as f64 * CHUNK as f64 / RATE as f64;
    let (mut pieces, mut start, mut quiet) = (Vec::<Range<usize>>::new(), None::<usize>, 0usize);
    let close = |from: usize, to: usize, pieces: &mut Vec<Range<usize>>| {
        if seconds(to - from) >= LEAST_SECONDS {
            pieces.push(from * CHUNK..to * CHUNK);
        }
    };
    for (i, probability) in probabilities.iter().enumerate() {
        match start {
            None if *probability >= ON => (start, quiet) = (Some(i), 0),
            Some(from) if *probability < OFF => {
                quiet += 1;
                if seconds(quiet) >= QUIET_SECONDS {
                    close(from, i + 1 - quiet, &mut pieces);
                    start = None;
                }
            }
            Some(_) => quiet = 0,
            None => {}
        }
    }
    if let Some(from) = start {
        close(from, probabilities.len() - quiet, &mut pieces);
    }
    let (pad, join) = ((PAD_SECONDS * RATE as f64) as usize, (JOIN_SECONDS * RATE as f64) as usize);
    let mut joined: Vec<Range<usize>> = Vec::new();
    for piece in pieces {
        let piece = piece.start.saturating_sub(pad)..(piece.end + pad).min(length);
        match joined.last_mut() {
            Some(last) if piece.start <= last.end + join => last.end = last.end.max(piece.end),
            _ => joined.push(piece),
        }
    }
    joined
}

/// The ONNX Runtime library beside the program that runs.
pub fn runtime_beside_program() -> Result<std::path::PathBuf, String> {
    let name = format!("{}onnxruntime{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    let exe = std::env::current_exe().map_err(err)?;
    let dir = exe.parent().ok_or("the program has no folder")?;
    Some(dir.join(&name)).filter(|path| path.is_file()).ok_or_else(|| format!("{name} is not in {}", dir.display()))
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Probabilities of 32 ms steps: `speech` steps at 0.9 between `quiet` ones at 0.05.
    fn steps(runs: &[(usize, f32)]) -> Vec<f32> {
        runs.iter().flat_map(|&(count, p)| std::iter::repeat_n(p, count)).collect()
    }

    #[test]
    fn speech_is_found_padded_and_joined_over_short_gaps() {
        // 1 s quiet, 2 s speech, 0.6 s quiet, 1 s speech, 3 s quiet, 0.1 s speech (too short), 1 s quiet.
        let probabilities = steps(&[(31, 0.05), (63, 0.9), (19, 0.05), (31, 0.9), (94, 0.05), (3, 0.9), (31, 0.05)]);
        let length = probabilities.len() * CHUNK;
        let found = spans(&probabilities, length);
        assert_eq!(found.len(), 1, "the two pieces 0.6 s apart are one, the 0.1 s blip is no speech: {found:?}");
        let (start, end) = (found[0].start as f64 / RATE as f64, found[0].end as f64 / RATE as f64);
        assert!((start - (0.992 - 0.2)).abs() < 0.05, "starts 0.2 s before the speech: {start}");
        assert!((end - (4.608 + 0.2)).abs() < 0.05, "ends 0.2 s after it: {end}");
    }

    #[test]
    fn a_sound_of_no_speech_has_none_and_speech_to_the_end_is_kept() {
        assert!(spans(&steps(&[(100, 0.1)]), 100 * CHUNK).is_empty());
        let found = spans(&steps(&[(20, 0.1), (40, 0.95)]), 60 * CHUNK);
        assert_eq!(found.last().map(|span| span.end), Some(60 * CHUNK), "speech running to the end ends with the sound");
    }
}
