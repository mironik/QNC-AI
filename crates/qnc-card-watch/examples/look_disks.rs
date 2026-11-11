//! What every attached disk is, by the camera catalog of a QNC root: `look_disks <QNC root>`.
//! Reads only (the configuration, the catalog and the disks).

fn main() -> Result<(), String> {
    let root = std::env::args().nth(1).ok_or("look_disks <QNC root>")?;
    let catalog = qnc_card_watch::catalog(std::path::Path::new(&root))?;
    for volume in qnc_source_volumes::mounted() {
        let started = std::time::Instant::now();
        let found = qnc_card_watch::what_is(&catalog, volume);
        println!("{:?} ({} ms)", found, started.elapsed().as_millis());
    }
    Ok(())
}
