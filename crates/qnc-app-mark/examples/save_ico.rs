//! Writes the mark as a Windows icon (16 to 256 pixels in one file) for the shortcuts on the desktop
//! and in the Start menu (user 2026-11-18): `save_ico <file.ico> [RRGGBB]`, the colour behind the
//! letters (black when not given; QNC, QNC Wizard and QNC Tray differ only in it).

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let file = args.next().ok_or("save_ico <file.ico> [RRGGBB]")?;
    let back = match args.next() {
        Some(hex) => {
            let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).map_err(|e| format!("colour {hex}: {e}"))?;
            [(value >> 16) as u8, (value >> 8) as u8, value as u8]
        }
        None => [0, 0, 0],
    };
    let frames = [16u32, 24, 32, 48, 64, 128, 256]
        .into_iter()
        .map(|size| image::codecs::ico::IcoFrame::as_png(&qnc_app_mark::qnc_mark_on(size, back), size, size, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let out = std::fs::File::create(&file).map_err(|e| e.to_string())?;
    image::codecs::ico::IcoEncoder::new(out).encode_images(&frames).map_err(|e| e.to_string())?;
    println!("{file}");
    Ok(())
}
