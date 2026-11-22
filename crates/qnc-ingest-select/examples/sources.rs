//! Reads (only) the sources of this computer as Ingest does and lists the roots of a tab.
//! Usage: sources QNC_ROOT [local|lan|intranet]
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::path::PathBuf::from(args.first().ok_or("usage: sources QNC_ROOT [tab]")?);
    let tab = args.get(1).map_or("lan", String::as_str);
    let config = qnc_ingest_select::SelectionConfig::load(&root).map_err(|e| e.to_string())?;
    let mut browser = config.browser().map_err(|e| e.to_string())?;
    for entry in browser.roots(tab)?.entries {
        println!("{} {}", entry.name, entry.qnc_uri);
    }
    Ok(())
}
