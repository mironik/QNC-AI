use super::*;
use rusqlite::Connection;

const PROJECT_DB: &str = "qnc://local/db/project_db/p1";

fn project() -> (tempfile::TempDir, ProjectDbTarget) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("project.db");
    Connection::open(&file)
        .unwrap()
        .execute_batch(
            "CREATE TABLE project_settings (project_id TEXT);
             INSERT INTO project_settings (project_id) VALUES ('p1');
             CREATE VIEW public_project_settings AS SELECT project_id FROM project_settings;",
        )
        .unwrap();
    (dir, ProjectDbTarget::from_owner_binding(&file, PROJECT_DB).unwrap())
}

fn entry(clip: &str, label: &str, at: u64) -> NewEntry {
    NewEntry { clip_id: clip.into(), kind: "source_marks".into(), label: label.into(), state: serde_json::json!({ "in": at }) }
}

fn wait(history: &mut ClipHistory, target: &ProjectDbTarget, clip: &str, count: usize) -> Vec<Entry> {
    for _ in 0..1000 {
        if let Some(entries) = history.for_clip(Some(target), Some(clip)) {
            if entries.len() == count && entries.iter().all(|entry| entry.id > 0) {
                return entries;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("not read")
}

/// User 2026-11-25: the history is kept per clip, not together; each entry keeps its owner's state.
#[test]
fn the_history_is_kept_per_clip_the_oldest_first_with_its_state() {
    let (_dir, target) = project();
    assert!(read(&target, "a").unwrap().is_empty(), "a project without the table has no history");
    let mut history = ClipHistory::new();
    history.add(&target, entry("a", "IN 10", 10));
    history.add(&target, entry("b", "IN 99", 99));
    history.add(&target, entry("a", "IN 12", 12));
    let a = wait(&mut history, &target, "a", 2);
    assert_eq!(a.iter().map(|entry| entry.label.as_str()).collect::<Vec<_>>(), ["IN 10", "IN 12"]);
    assert_eq!(a[1].state, serde_json::json!({ "in": 12 }));
    assert_eq!(read(&target, "b").unwrap().len(), 1, "another clip has its own history");
}

#[test]
fn an_entry_without_a_clip_or_a_label_is_refused() {
    let (_dir, target) = project();
    let mut client = target.open(Access::ReadWrite, vec![EditHistoryModule::factory()]).unwrap();
    let refused = client.execute(MODULE_ID, encode(&Operation::Add(entry("a", "", 1))).unwrap());
    assert!(refused.is_err());
}
