//! The few functions of the sherpa-onnx C API (2.0.2) a streaming transducer needs, loaded from
//! the library beside this program. The recognizer configuration is zeroed and only its leading
//! fields are set (the C API takes every zero field as its default); the result is read through
//! its leading fields only.

use libloading::{Library, Symbol};
use std::{
    ffi::{c_char, CStr, CString},
    path::Path,
};

/// The leading fields of `SherpaOnnxOnlineRecognizerConfig` (feature config, then the model
/// config up to `tokens_buf_size`), followed by zeroes for everything after them.
#[repr(C)]
struct Config {
    sample_rate: i32,
    feature_dim: i32,
    transducer_encoder: *const c_char,
    transducer_decoder: *const c_char,
    transducer_joiner: *const c_char,
    paraformer_encoder: *const c_char,
    paraformer_decoder: *const c_char,
    zipformer2_ctc_model: *const c_char,
    tokens: *const c_char,
    num_threads: i32,
    provider: *const c_char,
    debug: i32,
    model_type: *const c_char,
    modeling_unit: *const c_char,
    bpe_vocab: *const c_char,
    tokens_buf: *const c_char,
    tokens_buf_size: i32,
    /// The rest of the C struct, zero (larger than it).
    rest: [u8; 4096],
}

/// The leading fields of `SherpaOnnxOnlineRecognizerResult`.
#[repr(C)]
struct RawResult {
    text: *const c_char,
    tokens: *const c_char,
    tokens_arr: *const *const c_char,
    timestamps: *const f32,
    count: i32,
    json: *const c_char,
}

#[repr(C)]
struct Opaque {
    _private: [u8; 0],
}

pub use qnc_transcribe_jobs::Token;

/// How long the last token of a window lasts (the model's frame step, 80 ms): a streaming result
/// gives where each token starts, not how long it is.
const LAST_TOKEN: f32 = 0.08;

/// The sherpa-onnx library and one streaming recognizer of a model.
pub struct Recognizer {
    library: Library,
    recognizer: *const Opaque,
}

// The recognizer is used from the one job thread of this program.
unsafe impl Send for Recognizer {}

const LIBRARY: &str = if cfg!(windows) {
    "sherpa-onnx-c-api.dll"
} else if cfg!(target_os = "macos") {
    "libsherpa-onnx-c-api.dylib"
} else {
    "libsherpa-onnx-c-api.so"
};

impl Recognizer {
    /// The recognizer of the streaming transducer in `folder` (encoder, decoder, joiner, tokens).
    pub fn open(folder: &Path, threads: u32) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let path = exe.parent().ok_or("the program has no folder")?.join(LIBRARY);
        // SAFETY: the library is sherpa-onnx's C API, loaded from beside this program.
        let library = unsafe { Library::new(&path) }.map_err(|e| format!("{} is not beside the program: {e}", path.display()))?;
        let file = |prefix: &str, suffix: &str| -> Result<CString, String> {
            let entries = std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
            let mut found: Vec<_> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix) && n.ends_with(suffix)))
                .collect();
            found.sort();
            let path = found.into_iter().next().ok_or_else(|| format!("{}: no {prefix}*{suffix}", folder.display()))?;
            CString::new(path.to_string_lossy().into_owned()).map_err(|e| e.to_string())
        };
        let (encoder, decoder, joiner, tokens) = (file("encoder", ".onnx")?, file("decoder", ".onnx")?, file("joiner", ".onnx")?, file("tokens", ".txt")?);
        let provider = CString::new("cpu").unwrap();
        // SAFETY: every field the C API reads is set or zero (its default). The model type is
        // found from the model itself (a NeMo transducer by its decoder's outputs).
        let mut config: Config = unsafe { std::mem::zeroed() };
        config.sample_rate = 16_000;
        config.feature_dim = 80;
        config.transducer_encoder = encoder.as_ptr();
        config.transducer_decoder = decoder.as_ptr();
        config.transducer_joiner = joiner.as_ptr();
        config.tokens = tokens.as_ptr();
        config.num_threads = threads.max(1) as i32;
        config.provider = provider.as_ptr();
        let recognizer = unsafe {
            let create: Symbol<unsafe extern "C" fn(*const Config) -> *const Opaque> = symbol(&library, b"SherpaOnnxCreateOnlineRecognizer\0")?;
            create(&config)
        };
        if recognizer.is_null() {
            return Err(format!("sherpa-onnx could not load the model in {}", folder.display()));
        }
        Ok(Self { library, recognizer })
    }

    /// The tokens of `samples` (mono, -1..1, `rate` Hz) heard in `language` (`hr`, `en`... or
    /// `auto`; the model maps it to its prompt).
    pub fn decode(&self, samples: &[f32], rate: u32, language: &str) -> Result<Vec<Token>, String> {
        let n = i32::try_from(samples.len()).map_err(|_| "too much sound for one decode")?;
        let (key, value) = (CString::new("language").unwrap(), CString::new(language).map_err(|e| e.to_string())?);
        unsafe {
            let create: Symbol<unsafe extern "C" fn(*const Opaque) -> *const Opaque> = symbol(&self.library, b"SherpaOnnxCreateOnlineStream\0")?;
            let option: Symbol<unsafe extern "C" fn(*const Opaque, *const c_char, *const c_char)> = symbol(&self.library, b"SherpaOnnxOnlineStreamSetOption\0")?;
            let accept: Symbol<unsafe extern "C" fn(*const Opaque, i32, *const f32, i32)> = symbol(&self.library, b"SherpaOnnxOnlineStreamAcceptWaveform\0")?;
            let finished: Symbol<unsafe extern "C" fn(*const Opaque)> = symbol(&self.library, b"SherpaOnnxOnlineStreamInputFinished\0")?;
            let ready: Symbol<unsafe extern "C" fn(*const Opaque, *const Opaque) -> i32> = symbol(&self.library, b"SherpaOnnxIsOnlineStreamReady\0")?;
            let decode: Symbol<unsafe extern "C" fn(*const Opaque, *const Opaque)> = symbol(&self.library, b"SherpaOnnxDecodeOnlineStream\0")?;
            let result: Symbol<unsafe extern "C" fn(*const Opaque, *const Opaque) -> *const RawResult> = symbol(&self.library, b"SherpaOnnxGetOnlineStreamResult\0")?;
            let free_result: Symbol<unsafe extern "C" fn(*const RawResult)> = symbol(&self.library, b"SherpaOnnxDestroyOnlineRecognizerResult\0")?;
            let free_stream: Symbol<unsafe extern "C" fn(*const Opaque)> = symbol(&self.library, b"SherpaOnnxDestroyOnlineStream\0")?;
            let stream = create(self.recognizer);
            if stream.is_null() {
                return Err("sherpa-onnx gave no stream".into());
            }
            option(stream, key.as_ptr(), value.as_ptr());
            accept(stream, rate as i32, samples.as_ptr(), n);
            // A little silence after the sound, so its last words come out of the last chunk.
            let tail = vec![0f32; rate as usize / 2];
            accept(stream, rate as i32, tail.as_ptr(), tail.len() as i32);
            finished(stream);
            while ready(self.recognizer, stream) != 0 {
                decode(self.recognizer, stream);
            }
            let raw = result(self.recognizer, stream);
            let read = if raw.is_null() { Err("sherpa-onnx gave no result".to_string()) } else { Ok(read_result(&*raw)) };
            if !raw.is_null() {
                free_result(raw);
            }
            free_stream(stream);
            read
        }
    }
}

impl Drop for Recognizer {
    fn drop(&mut self) {
        unsafe {
            if let Ok(destroy) = symbol::<unsafe extern "C" fn(*const Opaque)>(&self.library, b"SherpaOnnxDestroyOnlineRecognizer\0") {
                destroy(self.recognizer);
            }
        }
    }
}

unsafe fn symbol<'l, T>(library: &'l Library, name: &[u8]) -> Result<Symbol<'l, T>, String> {
    library.get(name).map_err(|e| format!("sherpa-onnx has no {}: {e}", String::from_utf8_lossy(&name[..name.len() - 1])))
}

/// The tokens of a result, each lasting until the next one starts.
unsafe fn read_result(raw: &RawResult) -> Vec<Token> {
    let text = |p: *const c_char| if p.is_null() { String::new() } else { CStr::from_ptr(p).to_string_lossy().into_owned() };
    let count = raw.count.max(0) as usize;
    if raw.tokens_arr.is_null() || raw.timestamps.is_null() {
        return Vec::new();
    }
    let starts: Vec<f32> = (0..count).map(|i| *raw.timestamps.add(i)).collect();
    (0..count)
        .map(|i| Token {
            text: text(*raw.tokens_arr.add(i)),
            start: starts[i],
            duration: starts.get(i + 1).map_or(LAST_TOKEN, |next| (next - starts[i]).max(0.0)),
            log_prob: None,
        })
        .collect()
}
