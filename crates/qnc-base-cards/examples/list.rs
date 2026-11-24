//! The cards of the Ingest bases of a QNC root (read only): `list <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: list <root>")?);
    let folder = qnc_base_cards::folder(&root)?;
    println!("folder: {}", folder.display());
    let listed = qnc_base_cards::cards(&root)?;
    for card in &listed.cards {
        println!("{} {} clips={}", card.base, card.source_uri, card.clips);
    }
    for skipped in &listed.skipped {
        println!("skipped: {skipped}");
    }
    Ok(())
}
