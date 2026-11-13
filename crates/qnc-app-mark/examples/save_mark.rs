//! Saves the mark as PNG files to look at: `save_mark <folder>` (16, 20, 24, 32, 64 and 256 pixels).

fn main() -> Result<(), String> {
    let folder = std::env::args().nth(1).ok_or("save_mark <folder>")?;
    for size in [16u32, 20, 24, 32, 64, 256] {
        let pixels = qnc_app_mark::qnc_mark(size);
        let picture = image::RgbaImage::from_raw(size, size, pixels).ok_or("pixels")?;
        picture.save(std::path::Path::new(&folder).join(format!("qnc-mark-{size}.png"))).map_err(|e| e.to_string())?;
    }
    Ok(())
}
