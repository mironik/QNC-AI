//! Speech to text as a plug-in program (contract `qnc.transcribe` 2) with NVIDIA Nemotron 3.5 ASR
//! Streaming through sherpa-onnx's streaming recognizer, on the processor (user 2026-11-19: the
//! live transcript while a clip plays; wav2vec2 did not keep up beside the player). The model is
//! cache-aware: it computes only the sound that is new, a chunk of 560 ms at a time, so a few
//! seconds of speech take a fraction of that on two cores (measured: 60 s in 16 s, 20 s beside a
//! decode of the original). The language of the job is the model's prompt (hr -> hr-HR). The
//! sherpa-onnx C library (2.0.2 or later) is loaded from beside this program; without it the
//! program answers an error, nothing falls back. Normal priority: it is heard live.

mod engine;
mod sherpa;

fn main() {
    qnc_dev_diagnostics::log_panics("qnc-transcribe-nemotron");
    let mut engine = engine::Nemotron::default();
    if let Err(error) = qnc_transcribe_jobs::serve(&mut engine, "qnc.transcribe.nemotron", env!("CARGO_PKG_VERSION"), "sherpa-onnx nemotron-3.5-asr-streaming") {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
