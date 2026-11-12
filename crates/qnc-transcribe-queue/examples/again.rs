//! Transcribes one clip of the active project again, whole, with the fast engine, through the
//! project's transcription queue (the same runner the background worker uses): the new
//! transcript replaces the old over its range. Usage: again QNC_ROOT CLIP_ID

use std::{path::Path, sync::Arc, time::Duration};

use qnc_transcript_db::{QueueQuality, QueueState, Queued};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, clip] = args.as_slice() else { return Err("usage: again QNC_ROOT CLIP_ID".into()) };
    let root = Path::new(root);
    let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
    let snapshot = active.read().map_err(|e| e.to_string())?;
    let reader = active.settings_reader().clone();
    let settings = snapshot.settings;
    println!("project {}", snapshot.project_uri);
    let db = qnc_db_broker::ProjectDbTarget::for_project(&reader, &settings)?;
    let entry = qnc_transcript_db::read_queue(&db)?.into_iter().find(|entry| &entry.clip_id == clip).ok_or("the clip is not in the queue")?;
    qnc_transcript_db::enqueue(&db, vec![Queued { state: QueueState::Waiting, error: String::new(), took_ms: 0, quality: QueueQuality::Fast, ..entry }])?;
    let sources = qnc_source_bindings::load(root)?.sources;
    let target = qnc_content_store::ContentTarget::for_project(&reader, &settings).map_err(|e| e.to_string());
    let clips = Arc::new(qnc_player_clip_content::PlayerClipContent { target });
    let locator = qnc_still_locator::StillLocator::new(reader, settings, sources, clips);
    let mut runner = qnc_transcribe_queue::QueueRunner::new();
    let started = std::time::Instant::now();
    loop {
        runner.ensure(&locator, &db);
        let _ = runner.take();
        let state = qnc_transcript_db::read_queue(&db)?.into_iter().find(|entry| &entry.clip_id == clip).map(|entry| (entry.state, entry.error));
        match state {
            Some((QueueState::Done, _)) => break,
            Some((QueueState::Failed, error)) => return Err(error),
            _ => std::thread::sleep(Duration::from_secs(1)),
        }
        if started.elapsed() > Duration::from_secs(900) {
            return Err("not done in 15 min".into());
        }
    }
    println!("done in {:.0} s", started.elapsed().as_secs_f64());
    Ok(())
}
