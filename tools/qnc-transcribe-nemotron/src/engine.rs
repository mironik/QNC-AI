//! A job: the WAV in windows of at most 30 s (cut at the quietest tenth of a second of its last
//! five, as Parakeet's), each window fed to a stream of the streaming recognizer with the job's
//! language, its tokens joined into words and the words into segments (`qnc_transcribe_jobs`'s
//! one rule).

use crate::sherpa::Recognizer;
use qnc_transcribe_jobs::{segments, token_words, Backend, Engine, Segment, Transcribe, WHISPER_RATE};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

const WINDOW_SECONDS: usize = 30;

#[derive(Default)]
pub struct Nemotron {
    kept: Option<(PathBuf, u32, Recognizer)>,
}

impl Engine for Nemotron {
    fn backends(&self) -> Vec<Backend> {
        vec![Backend::Cpu]
    }

    fn transcribe(&mut self, job: &Transcribe, cancel: &AtomicBool, on_segment: &mut dyn FnMut(Segment) -> Result<(), String>) -> Result<String, String> {
        if job.backend != Backend::Cpu {
            return Err("this program runs Nemotron on the processor only".into());
        }
        if self.kept.as_ref().is_none_or(|(model, threads, _)| *model != job.model || *threads != job.threads) {
            self.kept = None;
            self.kept = Some((job.model.clone(), job.threads, Recognizer::open(&job.model, job.threads)?));
        }
        let recognizer = &self.kept.as_ref().expect("kept above").2;
        let samples = &job.samples;
        let rate = WHISPER_RATE as usize;
        for window in qnc_transcribe_jobs::quiet_pieces(samples, rate, WINDOW_SECONDS) {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            let tokens = recognizer.decode(&samples[window.clone()], WHISPER_RATE, &job.language)?;
            for segment in segments(&token_words(&tokens, window.start as f64 / rate as f64)) {
                on_segment(segment)?;
            }
        }
        Ok(job.language.clone())
    }
}
