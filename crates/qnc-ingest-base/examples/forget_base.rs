//! Deletes the Ingest bases of a QNC-AI root whose label (time and number) starts with the
//! given text: `forget_base <root> <label start>`. Only bases, never a story's project.

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (Some(root), Some(start)) = (args.next(), args.next()) else {
        return Err("usage: forget_base <root> <label start>".into());
    };
    let root = std::path::PathBuf::from(root);
    for base in qnc_ingest_base::all(&root)?.into_iter().filter(|base| base.label().starts_with(&start)) {
        qnc_ingest_base::forget(&root, &base.id)?;
        println!("deleted {}", base.name);
    }
    Ok(())
}
