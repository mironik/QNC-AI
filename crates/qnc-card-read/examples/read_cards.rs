//! Reads every attached card of a known camera and prints what it holds (read only: nothing is
//! kept, nothing is written). `cargo run --release -p qnc-card-read --example read_cards`

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = match qnc_card_watch::catalog(&root) {
        Ok(catalog) => catalog,
        Err(error) => return eprintln!("no camera catalog: {error}"),
    };
    for volume in qnc_source_volumes::mounted() {
        if !matches!(qnc_card_watch::what_is(&catalog, volume.clone()), qnc_card_watch::Found::Card { .. }) {
            continue;
        }
        let started = std::time::Instant::now();
        match qnc_card_read::read(&root, &volume) {
            Ok(content) => {
                println!("{} {}: {} clips, {} warnings, {:?}", content.disk, content.serial, content.clips.len(), content.warnings.len(), started.elapsed());
                for clip in content.clips.iter().take(5) {
                    println!("  {} {:?} s, proxies {}, thumbnail {}", clip.name, clip.duration_seconds, clip.proxies.len(), clip.thumbnail.is_some());
                }
                for warning in content.warnings.iter().take(5) {
                    println!("  ! {warning}");
                }
            }
            Err(error) => println!("{}: {error}", volume.root.display()),
        }
    }
}
