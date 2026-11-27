//! The records of a base's database copied into a story's database: `copy_records BASE_DB STORY_DB`
//! (each with its ai.db beside it). For copies of databases.

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (base, story) = (args.next().ok_or("BASE_DB")?, args.next().ok_or("STORY_DB")?);
    let copied = qnc_ingest_base::copy_base_records(std::path::Path::new(&base), std::path::Path::new(&story))?;
    println!("copied {copied} rows");
    Ok(())
}
