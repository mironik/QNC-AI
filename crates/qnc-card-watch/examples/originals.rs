//! The originals on each card attached now and in the bases that have the card (read only):
//! `originals <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: originals <root>")?);
    let catalog = qnc_card_watch::catalog(&root)?;
    for volume in qnc_source_volumes::mounted() {
        let Ok(card) = qnc_card_watch::card_originals(&catalog, &volume) else { continue };
        if card.is_empty() {
            continue;
        }
        println!("{} {} originals={}", volume.root.display(), volume.source_uri(), card.len());
        println!("  first on card: {}", card[0]);
    }
    Ok(())
}
