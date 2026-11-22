//! Reads (only) the sources of this computer as Ingest does, lists the roots of a tab and opens
//! the first. Usage: sources QNC_ROOT [local|lan|intranet]
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::path::PathBuf::from(args.first().ok_or("usage: sources QNC_ROOT [tab]")?);
    let tab = args.get(1).map_or("lan", String::as_str);
    let config = qnc_ingest_select::SelectionConfig::load(&root).map_err(|e| e.to_string())?;
    let mut browser = config.browser().map_err(|e| e.to_string())?;
    let roots = browser.roots(tab)?.entries;
    for entry in &roots {
        println!("{} {}", entry.name, entry.qnc_uri);
    }
    if let Some(first) = roots.first() {
        match browser.open(&first.qnc_uri) {
            Ok(state) => state.entries.iter().for_each(|entry| println!("  {} {}", entry.name, entry.qnc_uri)),
            Err(error) => println!("  open: {error}"),
        }
    }
    Ok(())
}
