//! Prints the active project and what is done in the active Ingest base, or in base `ID` (read
//! only). `progress ROOT [ID]`.

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("the QNC root"));
    let active = qnc_active_project_read::ActiveProjectReader::from_root(&root).and_then(|active| active.read().map_err(Into::into));
    println!("active: {:?}", active.map(|snapshot| snapshot.settings.project_id).map_err(|error| error.to_string()));
    println!("not taken: {:?}", qnc_ingest_base::not_taken(&root).map(|clips| clips.len()));
    match std::env::args().nth(2) {
        Some(id) => println!("{:?}", qnc_work_progress::progress_of(&root, &id)),
        None => println!("{:?}", qnc_work_progress::progress(&root)),
    }
}
