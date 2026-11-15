//! Writes the card index of the base in a folder: `card_index <base folder>` (use a copy).
fn main() {
    let folder = std::env::args().nth(1).expect("a base folder");
    println!("{:?}", qnc_ingest_base::write_card_index_in(std::path::Path::new(&folder)));
}
