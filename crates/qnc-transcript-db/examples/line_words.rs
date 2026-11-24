//! Reads (only) one clip's transcripts from a project database and tells, per line, how many
//! words its text has and how many timed words it carries. Usage: line_words PROJECT_DB PROJECT_ID CLIP FROM TO

use qnc_db_broker::ProjectDbTarget;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, id, clip, from, to] = args.as_slice() else { return Err("usage: line_words PROJECT_DB PROJECT_ID CLIP FROM TO".into()) };
    let (from, to): (u64, u64) = (from.parse().map_err(|_| "FROM")?, to.parse().map_err(|_| "TO")?);
    let target = ProjectDbTarget::from_owner_binding(std::path::Path::new(file), &format!("qnc://local/db/project_db/{id}"))?;
    for transcript in qnc_transcript_db::read_clip(&target, clip)? {
        println!("[{}..{}] {}", transcript.from_frame, transcript.to_frame, transcript.engine);
        for line in transcript.lines.iter().filter(|line| line.to >= from && line.from <= to) {
            println!("  {}-{} text {} timed {}", line.from, line.to, line.text.split_whitespace().count(), line.words.len());
            let timed: Vec<&str> = line.words.iter().map(|word| word.text.as_str()).collect();
            println!("    {}", timed.join(" "));
        }
    }
    Ok(())
}
