//! Writes the mark as a Windows icon (16 to 256 pixels in one file) for the shortcuts on the desktop
//! and in the Start menu (user 2026-11-18): `save_ico <file.ico>`.

fn main() -> Result<(), String> {
    let file = std::env::args().nth(1).ok_or("save_ico <file.ico>")?;
    let frames = [16u32, 24, 32, 48, 64, 128, 256]
        .into_iter()
        .map(|size| image::codecs::ico::IcoFrame::as_png(&qnc_app_mark::qnc_mark(size), size, size, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let out = std::fs::File::create(&file).map_err(|e| e.to_string())?;
    image::codecs::ico::IcoEncoder::new(out).encode_images(&frames).map_err(|e| e.to_string())?;
    println!("{file}");
    Ok(())
}
