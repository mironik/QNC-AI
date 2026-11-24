//! Shot analysis of one MXF original, printed (the file is only read; nothing is written but
//! the decoder's small pictures in the system temp folder, removed after). With a folder, the 13
//! filmstrip thumbnails made in the same pass and the strip of every picture are written there.
//! `cargo run --release -p qnc-shot-analysis --example analyse_file -- <file.mxf> [h264|mpeg2video] [folder]`

use std::sync::atomic::AtomicBool;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = std::path::PathBuf::from(args.next().expect("an MXF file"));
    let codec = args.next().unwrap_or_else(|| "h264".into());
    let folder = args.next().map(std::path::PathBuf::from);
    let cancel = AtomicBool::new(false);
    let pace = qnc_shot_analysis::Pace { player_plays: &|| false, wanted: &|| None, takes: &|_| true, workers: &qnc_shot_analysis::most_workers, cancel: &cancel };
    let frames = qnc_shot_analysis::KeyPictures::open(&path).expect("index").expect("an index").frames;
    // The filmstrip's 13 places, spread over the clip.
    let targets: Vec<u64> = (0..13u64).map(|i| i * frames.saturating_sub(1) / 12).collect();
    let started = std::time::Instant::now();
    let found = qnc_shot_analysis::analyse_file_with("example", &codec, &path, &pace, &targets).expect("analysis");
    println!("pictures={} shots={} parts={} filmstrip={:?} ms={}", found.pictures, found.shots.len(), found.parts.len(), found.filmstrip.as_ref().map(Vec::len), started.elapsed().as_millis());
    if let (Some(folder), Some(jpegs)) = (folder, found.filmstrip.as_ref()) {
        std::fs::create_dir_all(&folder).unwrap();
        for (i, jpeg) in jpegs.iter().enumerate() {
            std::fs::write(folder.join(format!("{i:02}.jpg")), jpeg).unwrap();
        }
        let strip = qnc_shot_analysis::write_strip(&folder, "qnc://local/example", "example", &found.decoded).expect("strip");
        let bytes: u64 = strip.pictures.iter().map(|p| u64::from(p.len)).sum();
        println!("strip pictures={} bytes={bytes}", strip.pictures.len());
    }
    for shot in found.shots {
        println!("shot {:>6} - {:>6}  key={:>6}  bright={:>5.1} sharp={:>8.1} motion={:.3}  {:?}", shot.from, shot.to, shot.key_frame, shot.brightness, shot.sharpness, shot.motion, shot.looks);
    }
    for part in found.parts {
        println!("part {:>6} - {:>6}  key={:>6}  {}", part.from, part.to, part.key_frame, part.camera);
    }
}
