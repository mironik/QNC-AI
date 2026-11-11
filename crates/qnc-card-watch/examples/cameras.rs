//! The camera of each card attached now (read only): `cameras <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: cameras <root>")?);
    let catalog = qnc_card_watch::catalog(&root)?;
    let (profiles, refused) = qnc_camera_profiles::read_profiles(&root.join(qnc_camera_profiles::PROFILE_DIR));
    for why in refused {
        println!("refused profile: {why}");
    }
    for volume in qnc_source_volumes::mounted() {
        let started = std::time::Instant::now();
        let camera = qnc_card_watch::camera_of(&catalog, &profiles, &volume);
        println!("{} {} {:?} {} ms", volume.root.display(), volume.serial_number, camera, started.elapsed().as_millis());
    }
    Ok(())
}
