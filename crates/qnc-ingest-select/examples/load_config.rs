//! Whether this computer's Select configuration loads (read only): `load_config <qnc root>`.
fn main() {
    let root = std::env::args().nth(1).expect("a QNC root");
    match qnc_ingest_select::SelectionConfig::load(std::path::Path::new(&root)) {
        Ok(config) => println!("ok: {} sources", config.sources.len()),
        Err(error) => println!("refused: {error}"),
    }
}
