//! Prints what is done in the active Ingest base (read only). `progress ROOT`.

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("the QNC root"));
    println!("{:?}", qnc_work_progress::progress(&root));
}
