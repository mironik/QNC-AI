//! Copies one file as the import does and says how fast (diagnostic; the source is only read).
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (source, destination) = (args.next().ok_or("usage: copy_speed <source> <destination>")?, args.next().ok_or("destination")?);
    let started = std::time::Instant::now();
    qnc_ingest_import_worker::copy_local_file(source.as_ref(), destination.as_ref())?;
    let seconds = started.elapsed().as_secs_f64();
    let bytes = std::fs::metadata(&destination).map_err(|e| e.to_string())?.len() as f64;
    println!("{:.0} MB in {seconds:.1} s = {:.1} MB/s", bytes / 1048576.0, bytes / 1048576.0 / seconds);
    Ok(())
}
