//! The records of a base copied into a story's project, as Create project does:
//! `copy_into_story QNC_ROOT BASE_ID STORY_ID`.

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (root, base, story) = (args.next().ok_or("QNC_ROOT")?, args.next().ok_or("BASE_ID")?, args.next().ok_or("STORY_ID")?);
    let copied = qnc_ingest_base::copy_base_into_story(std::path::Path::new(&root), &base, &story)?;
    println!("copied {copied} rows");
    Ok(())
}
