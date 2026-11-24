//! Lists (only reads) the root of a local source and, when given, a folder in it and a file's
//! size. Usage: list_root SOURCE_URI FOLDER [RELATIVE_FOLDER [RELATIVE_FILE]]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (uri, folder) = (&args[0], &args[1]);
    let reader = match qnc_source_reader::SourceReader::local(uri, folder) {
        Ok(reader) => reader,
        Err(error) => return println!("open: {error}"),
    };
    for relative in [Some("."), args.get(2).map(String::as_str)].into_iter().flatten() {
        let place = qnc_source_contract::SourceReference::new(uri, relative).unwrap();
        match reader.list(&place, 1000) {
            Ok(listing) => println!("{relative}: {}", listing.entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>().join(", ")),
            Err(error) => println!("{relative}: list: {error}"),
        }
    }
    if let Some(file) = args.get(3) {
        let place = qnc_source_contract::SourceReference::new(uri, file).unwrap();
        match reader.stat(&place) {
            Ok(info) => println!("{file}: {:?} bytes", info.byte_len),
            Err(error) => println!("{file}: {error}"),
        }
    }
}
