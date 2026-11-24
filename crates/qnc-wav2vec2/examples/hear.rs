//! What the model hears in a 16 kHz WAV (only read), timed: `cargo run --release -p qnc-wav2vec2
//! --example hear -- <model folder> <wav> [seconds of it] [times]`.

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (folder, wav) = (args.get(1).ok_or("model folder")?, args.get(2).ok_or("wav")?);
    let seconds: Option<f64> = args.get(3).and_then(|s| s.parse().ok());
    let times: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    let started = std::time::Instant::now();
    let model = qnc_wav2vec2::Wav2Vec2::load(std::path::Path::new(folder))?;
    let loaded = started.elapsed();
    let mut samples = qnc_transcribe_jobs::read_wav(std::path::Path::new(wav))?;
    if let Some(seconds) = seconds {
        samples.truncate((seconds * 16000.0) as usize);
    }
    for _ in 0..times {
        let started = std::time::Instant::now();
        let emissions = model.emissions(&samples)?;
        println!("load {loaded:?}, {:.1} s of speech in {:?}: {}", samples.len() as f64 / 16000.0, started.elapsed(), model.greedy(&emissions));
    }
    Ok(())
}
