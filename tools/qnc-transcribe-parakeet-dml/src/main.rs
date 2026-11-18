//! Speech to text as a plug-in program (contract `qnc.transcribe` 2) with NVIDIA Parakeet TDT
//! (fp32 ONNX) through ONNX Runtime, its encoder on the graphics card through DirectML (user
//! 2026-11-17: the transcription took most of the processor of an ingest; measured on Iris Xe,
//! the encoder 14.4 s of speech a second with the processor left free). The backend comes from
//! the AI catalog of the computer it runs on: `directml`, or `cpu` for the same model on the
//! processor. ONNX Runtime's DirectML build is in the folder `directml` beside the program (the
//! one beside it is sherpa-onnx's, for the processor). The model is loaded once and kept.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use qnc_parakeet_onnx::Parakeet;
use qnc_transcribe_jobs::{segments, token_words, Backend, Engine, Segment, Transcribe, WHISPER_RATE};

/// The longest window decoded at once (cut at a quiet place, so a word is rarely cut and a
/// cancel waits at most one window).
const WINDOW_SECONDS: usize = 30;
/// The folder of ONNX Runtime's DirectML build beside the program.
const DIRECTML_FOLDER: &str = "directml";

#[derive(Default)]
struct Program {
    kept: Option<(PathBuf, Backend, Parakeet)>,
}

impl Engine for Program {
    fn backends(&self) -> Vec<Backend> {
        vec![Backend::Directml, Backend::Cpu]
    }

    fn transcribe(&mut self, job: &Transcribe, cancel: &AtomicBool, on_segment: &mut dyn FnMut(Segment) -> Result<(), String>) -> Result<String, String> {
        if !matches!(job.backend, Backend::Directml | Backend::Cpu) {
            return Err("this program runs Parakeet on DirectML or on the processor".into());
        }
        if self.kept.as_ref().is_none_or(|(model, backend, _)| *model != job.model || *backend != job.backend) {
            self.kept = None;
            let directml = job.backend == Backend::Directml;
            let parakeet = Parakeet::load(&job.model, &runtime(directml)?, job.threads.max(1) as usize, directml)?;
            self.kept = Some((job.model.clone(), job.backend, parakeet));
        }
        let parakeet = &mut self.kept.as_mut().expect("kept above").2;
        let rate = WHISPER_RATE as usize;
        for window in qnc_transcribe_jobs::quiet_pieces(&job.samples, rate, WINDOW_SECONDS) {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            let tokens = parakeet.decode(&job.samples[window.clone()])?;
            let (features, encoder, decoder) = parakeet.last_ms;
            qnc_dev_diagnostics::log_line(
                qnc_dev_diagnostics::DiagnosticsStream::Player,
                format!("parakeet-dml window={:.1}s features_ms={features} encoder_ms={encoder} decoder_ms={decoder}", window.len() as f64 / rate as f64),
            );
            for segment in segments(&token_words(&tokens, window.start as f64 / rate as f64)) {
                on_segment(segment)?;
            }
        }
        // Parakeet v3 does not say its language: the one asked.
        Ok(job.language.clone())
    }
}

/// The ONNX Runtime library: its DirectML build in `directml` beside the program (DirectML's own
/// library loaded from there first, Windows keeps an older one), else the one beside it.
fn runtime(directml: bool) -> Result<PathBuf, String> {
    let name = format!("{}onnxruntime{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("the program has no folder")?;
    let dir = if directml { dir.join(DIRECTML_FOLDER) } else { dir.to_path_buf() };
    if directml {
        preload_directml(&dir)?;
    }
    Some(dir.join(&name)).filter(|path| path.is_file()).ok_or_else(|| format!("{name} is not in {}", dir.display()))
}

#[cfg(windows)]
fn preload_directml(dir: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let path = dir.join("DirectML.dll");
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let module = unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(wide.as_ptr()) };
    if module.is_null() {
        return Err(format!("{} could not be loaded", path.display()));
    }
    Ok(())
}

#[cfg(not(windows))]
fn preload_directml(_dir: &std::path::Path) -> Result<(), String> {
    Err("DirectML is Windows' own".into())
}

fn main() {
    qnc_dev_diagnostics::log_panics("qnc-transcribe-parakeet-dml");
    // Normal priority, not below as the processor's programs: it takes little of the processor but
    // needs it at once to give the card its work (audit 2026-11-18: below normal under a full
    // processor the card heard at 0.8x real time, idle at 9.5x).
    let mut program = Program::default();
    if let Err(error) = qnc_transcribe_jobs::serve(&mut program, "qnc.transcribe.parakeet-dml", env!("CARGO_PKG_VERSION"), "onnx runtime parakeet-tdt fp32") {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
