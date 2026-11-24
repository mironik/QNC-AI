//! NVIDIA Parakeet TDT 0.6B v3 as its fp32 ONNX export (CC-BY-4.0; the export of onnx-asr): the
//! mel features (`nemo128.onnx`), the encoder (`encoder-model.onnx`) and the decoder with its
//! joint (`decoder_joint-model.onnx`), with `vocab.txt`. The encoder carries nearly all the
//! work and runs on the graphics card through DirectML when asked (user 2026-11-17: measured on
//! Iris Xe, 14.4 s of speech a second with the processor left free); the features and the
//! decoder stay on the processor. Greedy TDT decoding: at every encoder frame the joint gives a
//! token and how many frames to go on. It reads the model and the samples it is given; it
//! writes nothing.

use std::path::Path;

use ort::session::{Session, SessionInputValue};
use ort::value::Tensor;
use qnc_transcribe_jobs::Token;

pub const MODULE_ID: &str = "qnc.module.parakeet-onnx";

/// Seconds of one encoder frame: the features' 10 ms hop times the encoder's subsampling of 8.
const FRAME_SECONDS: f32 = 0.08;
/// The state of the decoder's recurrent network: layers, batch, width.
const STATE: [usize; 3] = [2, 1, 640];
/// The width of an encoder frame.
const ENCODED: usize = 1024;
/// At most this many tokens at one frame before the decoding goes on (onnx-asr's rule).
const MOST_TOKENS_A_FRAME: usize = 10;

/// A loaded model, kept for the next job.
pub struct Parakeet {
    /// Milliseconds of the last decode: the features, the encoder, the decoding (for the log).
    pub last_ms: (u128, u128, u128),
    features: Session,
    encoder: Session,
    decoder: Session,
    /// The text of each token id; a word begins with a space.
    vocab: Vec<String>,
    blank: usize,
    /// The encoder is on the card: it is given only the fixed lengths of [`BUCKETS`].
    fixed: bool,
}

/// The lengths (feature frames, 100 a second) the encoder on the card is given: every 5 s up to
/// 31 s (a window is at most 30 s). The card's time follows the length given, not the speech in it
/// (measured on Iris Xe: 1 s for 10 s, 2 s for 20 s, 3 s for 31 s), so a short piece is laid in a
/// short length.
const BUCKETS: [usize; 6] = [500, 1000, 1500, 2000, 2500, 3100];

/// `features` ([128, frames]) laid in [128, `bucket`], the rest zeros.
fn padded(features: &[f32], frames: usize, bucket: usize) -> Vec<f32> {
    let mut laid = vec![0.0f32; 128 * bucket];
    for channel in 0..128 {
        laid[channel * bucket..channel * bucket + frames].copy_from_slice(&features[channel * frames..(channel + 1) * frames]);
    }
    laid
}

impl Parakeet {
    /// Loads the model of `folder` with the ONNX Runtime library `runtime` (its DirectML build when
    /// `directml`), the processor's parts on `threads` threads. A card DirectML cannot use is an
    /// error: nothing falls back to the processor.
    pub fn load(folder: &Path, runtime: &Path, threads: usize, directml: bool) -> Result<Self, String> {
        ort::init_from(runtime.to_string_lossy()).commit().map_err(err)?;
        let open = |name: &str, card: bool| -> Result<Session, String> {
            let builder = Session::builder().and_then(|builder| builder.with_intra_threads(threads));
            let builder = if card {
                // DirectML takes no parallel run and no memory pattern (ONNX Runtime's rule).
                builder.and_then(|builder| builder.with_parallel_execution(false)).and_then(|builder| builder.with_memory_pattern(false)).and_then(|builder| {
                    builder.with_execution_providers([ort::execution_providers::DirectMLExecutionProvider::default().build().error_on_failure()])
                })
            } else {
                builder
            };
            builder.and_then(|builder| builder.commit_from_file(folder.join(name))).map_err(|error| format!("{name}: {error}"))
        };
        let vocab = read_vocab(&folder.join("vocab.txt"))?;
        let blank = vocab.iter().position(|token| token == "<blk>").ok_or("vocab.txt has no <blk>")?;
        let mut parakeet = Self { last_ms: (0, 0, 0), features: open("nemo128.onnx", false)?, encoder: open("encoder-model.onnx", directml)?, decoder: open("decoder_joint-model.onnx", false)?, vocab, blank, fixed: directml };
        if directml {
            // Each length the card is given once, now (audit 2026-11-18: DirectML prepares its work
            // anew for every new length; the ingest's pieces were all of other lengths, so the card
            // prepared more than it heard, 1.3x real time instead of 11x).
            for frames in BUCKETS {
                parakeet.encode(vec![0.0; 128 * frames], frames, frames as i64)?;
            }
        }
        Ok(parakeet)
    }

    /// The encoder's frames of `features` ([1, 128, frames], `length` of them real), one vector a
    /// frame, and how many are real.
    fn encode(&mut self, features: Vec<f32>, frames: usize, length: i64) -> Result<(Vec<Vec<f32>>, usize), String> {
        let out = self
            .encoder
            .run(ort::inputs![
                "audio_signal" => Tensor::from_array(([1usize, 128, frames], features)).map_err(err)?,
                "length" => Tensor::from_array(([1usize], vec![length])).map_err(err)?
            ])
            .map_err(err)?;
        let (shape, data) = out["outputs"].try_extract_tensor::<f32>().map_err(err)?;
        let width = usize::try_from(shape[2]).map_err(err)?;
        let real = usize::try_from(out["encoded_lengths"].try_extract_tensor::<i64>().map_err(err)?.1[0]).map_err(err)?;
        Ok((transposed(data, width), real.min(width)))
    }

    /// The tokens of `samples` (16 kHz mono, -1..1), their times in seconds from its start.
    pub fn decode(&mut self, samples: &[f32]) -> Result<Vec<Token>, String> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }
        let started = std::time::Instant::now();
        let (features, frames) = {
            let out = self
                .features
                .run(ort::inputs![
                    "waveforms" => Tensor::from_array(([1usize, samples.len()], samples.to_vec())).map_err(err)?,
                    "waveforms_lens" => Tensor::from_array(([1usize], vec![samples.len() as i64])).map_err(err)?
                ])
                .map_err(err)?;
            let (shape, data) = out["features"].try_extract_tensor::<f32>().map_err(err)?;
            let frames = usize::try_from(shape[2]).map_err(err)?;
            let length = out["features_lens"].try_extract_tensor::<i64>().map_err(err)?.1[0];
            ((data.to_vec(), length), frames)
        };
        let featured = started.elapsed().as_millis();
        // On the card the features are laid in the smallest of a few fixed lengths (the rest zeros,
        // the real length given), so DirectML never prepares a new length.
        let (encoded, steps) = match self.fixed.then(|| BUCKETS.into_iter().find(|&bucket| bucket >= frames)).flatten() {
            Some(bucket) => self.encode(padded(&features.0, frames, bucket), bucket, features.1)?,
            None => self.encode(features.0, frames, features.1)?,
        };
        let encoded_at = started.elapsed().as_millis();
        let tokens = self.greedy(&encoded, steps)?;
        self.last_ms = (featured, encoded_at - featured, started.elapsed().as_millis() - encoded_at);
        Ok(tokens)
    }

    /// Greedy TDT: the joint at frame `t` gives a token (the blank says nothing) and a duration;
    /// a duration above zero moves on that many frames.
    fn greedy(&mut self, encoded: &[Vec<f32>], steps: usize) -> Result<Vec<Token>, String> {
        let mut state = (vec![0.0f32; STATE.iter().product()], vec![0.0f32; STATE.iter().product()]);
        let mut tokens: Vec<Token> = Vec::new();
        let mut last = self.blank as i32;
        let (mut t, mut at_frame) = (0usize, 0usize);
        while t < steps {
            let inputs: Vec<(std::borrow::Cow<str>, SessionInputValue)> = vec![
                ("encoder_outputs".into(), Tensor::from_array(([1usize, ENCODED, 1], encoded[t].clone())).map_err(err)?.into()),
                ("targets".into(), Tensor::from_array(([1usize, 1], vec![last])).map_err(err)?.into()),
                ("target_length".into(), Tensor::from_array(([1usize], vec![1i32])).map_err(err)?.into()),
                ("input_states_1".into(), Tensor::from_array((STATE, state.0.clone())).map_err(err)?.into()),
                ("input_states_2".into(), Tensor::from_array((STATE, state.1.clone())).map_err(err)?.into()),
            ];
            let out = self.decoder.run(inputs).map_err(err)?;
            let logits = out["outputs"].try_extract_tensor::<f32>().map_err(err)?.1;
            let (vocab, durations) = logits.split_at(self.vocab.len());
            let token = argmax(vocab);
            let step = argmax(durations);
            if token != self.blank {
                state = (
                    out["output_states_1"].try_extract_tensor::<f32>().map_err(err)?.1.to_vec(),
                    out["output_states_2"].try_extract_tensor::<f32>().map_err(err)?.1.to_vec(),
                );
                last = token as i32;
                tokens.push(Token {
                    text: self.vocab[token].clone(),
                    start: t as f32 * FRAME_SECONDS,
                    duration: step.max(1) as f32 * FRAME_SECONDS,
                    log_prob: Some(log_softmax_at(vocab, token)),
                });
                at_frame += 1;
            }
            if step > 0 {
                t += step;
                at_frame = 0;
            } else if token == self.blank || at_frame >= MOST_TOKENS_A_FRAME {
                t += 1;
                at_frame = 0;
            }
        }
        Ok(tokens)
    }
}

/// `vocab.txt`: one token and its id a line; SentencePiece's ▁ (a word begins) becomes a space.
fn read_vocab(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut vocab = Vec::new();
    for (line, entry) in text.lines().enumerate() {
        let (token, id) = entry.rsplit_once(' ').ok_or_else(|| format!("vocab.txt line {}: no id", line + 1))?;
        if id.parse::<usize>().ok() != Some(vocab.len()) {
            return Err(format!("vocab.txt line {}: id {id} out of order", line + 1));
        }
        vocab.push(token.replace('\u{2581}', " "));
    }
    Ok(vocab)
}

/// The encoder's [1, width, frames] as one vector of width a frame.
fn transposed(data: &[f32], frames: usize) -> Vec<Vec<f32>> {
    (0..frames).map(|t| (0..ENCODED).map(|c| data[c * frames + t]).collect()).collect()
}

fn argmax(values: &[f32]) -> usize {
    values.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map_or(0, |(i, _)| i)
}

fn log_softmax_at(values: &[f32], at: usize) -> f32 {
    let most = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let sum: f32 = values.iter().map(|v| (v - most).exp()).sum();
    values[at] - most - sum.ln()
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_major_encoder_output_is_read_frame_by_frame() {
        let frames = 3;
        let data: Vec<f32> = (0..ENCODED * frames).map(|i| i as f32).collect();
        let by_frame = transposed(&data, frames);
        assert_eq!(by_frame.len(), 3);
        assert_eq!(by_frame[1][0], 1.0);
        assert_eq!(by_frame[1][1], 4.0);
    }

    #[test]
    fn the_log_probability_of_the_best_of_two_equal_is_minus_ln_two() {
        assert!((log_softmax_at(&[1.0, 1.0], 0) + std::f32::consts::LN_2).abs() < 1e-6);
    }
}
