use super::*;
use std::sync::Arc;

const URI: &str = "qnc://local/db/project_db/p1";

/// A toy table module: one counter table it owns.
struct Counter;
struct CounterModule(rusqlite::Connection);

impl TableModuleFactory for Counter {
    fn id(&self) -> &'static str {
        "test.counter"
    }
    fn is_write(&self, payload: &Value) -> bool {
        payload == "add"
    }
    fn attach(&self, conn: rusqlite::Connection, access: Access) -> Result<Box<dyn TableModule>> {
        if access == Access::ReadWrite {
            conn.execute_batch("CREATE TABLE IF NOT EXISTS counter (n INTEGER)")
                .map_err(|e| e.to_string())?;
        }
        Ok(Box::new(CounterModule(conn)))
    }
}

impl TableModule for CounterModule {
    fn execute(&mut self, payload: Value) -> Result<Value> {
        if payload == "add" {
            let tx = begin_write(&mut self.0).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO counter VALUES (1)", [])
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        let n: i64 = self
            .0
            .query_row("SELECT count(*) FROM counter", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(Value::from(n))
    }
    fn connection(&self) -> &rusqlite::Connection {
        &self.0
    }
}

fn project(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    let file = dir.join("project.db");
    rusqlite::Connection::open(&file)
        .unwrap()
        .execute_batch(&format!(
            "CREATE TABLE project_settings (project_id TEXT);
             INSERT INTO project_settings VALUES ('{id}');
             CREATE VIEW public_project_settings AS SELECT project_id FROM project_settings;
             CREATE TABLE ingest_runtime (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at INTEGER NOT NULL);"
        ))
        .unwrap();
    file
}

fn modules() -> Vec<Arc<dyn TableModuleFactory>> {
    vec![Arc::new(Counter)]
}

#[test]
fn the_intermediary_serves_the_modules_it_is_given_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let file = project(dir.path(), "p1");
    let target = ProjectDbTarget::from_owner_binding(&file, URI).unwrap();
    let writer = ProjectDbWriter::start(target.clone(), modules()).unwrap();
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let writer = writer.clone();
            std::thread::spawn(move || {
                for _ in 0..5 {
                    writer.call("test.counter", Value::from("add")).unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let n: i64 = writer.request("test.counter", &"count").unwrap();
    assert_eq!(n, 20);
    let error = writer.call("test.unknown", Value::Null).unwrap_err();
    assert!(error.contains("Unknown project database module"), "{error}");
    let mut reader = target.open(Access::ReadOnly, modules()).unwrap();
    assert_eq!(
        reader
            .execute("test.counter", Value::from("count"))
            .unwrap(),
        20
    );
    assert!(
        reader.execute("test.counter", Value::from("add")).is_err(),
        "read-only"
    );
}

#[test]
fn only_the_database_of_that_project_is_opened_and_never_created() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("none.db");
    let target = ProjectDbTarget::from_owner_binding(&missing, URI).unwrap();
    assert!(target.open(Access::ReadWrite, modules()).is_err());
    assert!(!missing.exists(), "nothing creates a project database");
    let file = project(dir.path(), "other");
    let target = ProjectDbTarget::from_owner_binding(&file, URI).unwrap();
    let error = target.open(Access::ReadWrite, modules()).err().unwrap();
    assert!(error.contains("another project"), "{error}");
    let plain = dir.path().join("plain.db");
    rusqlite::Connection::open(&plain)
        .unwrap()
        .execute_batch("CREATE TABLE x (a)")
        .unwrap();
    let target = ProjectDbTarget::from_owner_binding(&plain, URI).unwrap();
    assert!(target.open(Access::ReadWrite, modules()).is_err());
}

#[test]
fn uris_come_from_the_workspace_of_the_project() {
    assert_eq!(
        project_db_uri("qnc://local/db/project_workspace/p1").unwrap(),
        "qnc://local/db/project_db/p1"
    );
    assert_eq!(project_id(URI).unwrap(), "p1");
    assert!(project_id("qnc://local/db/ingest_content/p1").is_err());
    assert!(project_db_uri("qnc://local/db/other/p1").is_err());
}

/// An idle writer lets the project database go, so a closed project can be deleted while
/// the application keeps its writer; the next request opens it again.
#[test]
fn an_idle_writer_releases_the_project_database_and_reopens_it() {
    let dir = tempfile::tempdir().unwrap();
    let file = project(dir.path(), "p1");
    let target = ProjectDbTarget::from_owner_binding(&file, URI).unwrap();
    let writer = ProjectDbWriter::start(target, modules()).unwrap();
    writer.call("test.counter", Value::from("add")).unwrap();
    std::thread::sleep(crate::transport::IDLE_RELEASE + std::time::Duration::from_millis(500));
    let moved = dir.path().join("moved.db");
    std::fs::rename(&file, &moved).expect("an idle writer holds no handle");
    std::fs::rename(&moved, &file).unwrap();
    assert_eq!(
        writer.call("test.counter", Value::from("add")).unwrap(),
        Value::from(2)
    );
}

/// A write made under a lease is made only while that lease is its owner's: once another
/// worker took it, the write is refused in the same transaction and nothing is written.
#[test]
fn a_write_under_a_lease_another_took_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let file = project(dir.path(), "p1");
    let hold = |owner: &str| {
        let conn = rusqlite::Connection::open(&file).unwrap();
        conn.pragma_update(None, "journal_mode", "PERSIST").unwrap(); // as the broker keeps it
        conn.execute(
                "INSERT OR REPLACE INTO ingest_runtime VALUES('worker', ?1, CAST(strftime('%s','now') AS INTEGER))",
                [qnc_write_lease::held_by(owner)],
            )
            .unwrap();
    };
    hold("a");
    let target = ProjectDbTarget::from_owner_binding(&file, URI).unwrap();
    let writer = ProjectDbWriter::start(target, modules()).unwrap();
    let leased = writer.leased(WriteLease {
        key: "worker".into(),
        owner: "a".into(),
        max_age_seconds: 5,
    });
    assert_eq!(
        leased.call("test.counter", Value::from("add")).unwrap(),
        Value::from(1)
    );
    hold("b");
    let error = leased.call("test.counter", Value::from("add")).unwrap_err();
    assert!(error.contains("lost its lease"), "{error}");
    assert_eq!(
        leased.call("test.counter", Value::from("count")).unwrap(),
        Value::from(1),
        "a read is not refused"
    );
    assert_eq!(
        writer.call("test.counter", Value::from("add")).unwrap(),
        Value::from(2),
        "a write without a lease still goes"
    );
}

/// A table module whose writes are edits: it writes only its own table (as every module) and the
/// intermediary keeps its writes in the journal of the project.
struct Notes(rusqlite::Connection);

#[derive(serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum NoteOp {
    Put(String),
    Count,
}

impl OwnedTables for Notes {
    const ID: &'static str = "test.notes";
    const JOURNALED: &'static [&'static str] = &["notes"];
    type Operation = NoteOp;
    type Data = i64;
    fn is_write(operation: &NoteOp) -> bool {
        matches!(operation, NoteOp::Put(_))
    }
    fn attach(conn: rusqlite::Connection, access: Access) -> Result<Self> {
        if access == Access::ReadWrite {
            conn.execute_batch("CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, text TEXT)").map_err(|e| e.to_string())?;
        }
        write_only_own_tables(&conn, access, &["notes"]);
        Ok(Self(conn))
    }
    fn execute(&mut self, operation: &NoteOp) -> Result<i64> {
        if let NoteOp::Put(text) = operation {
            let tx = begin_write(&mut self.0).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO notes (id, text) VALUES ('n', ?1) ON CONFLICT(id) DO UPDATE SET text = excluded.text", [text]).map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        self.0.query_row("SELECT count(*) FROM notes", [], |r| r.get(0)).map_err(|e| e.to_string())
    }
    fn connection(&self) -> &rusqlite::Connection {
        &self.0
    }
}

/// User 2026-11-25 (a quasi git of every edit): each write of an edit is one commit of the journal
/// of the project, with who wrote it, the piece and its action and the rows before and after; a
/// read is no commit. The module still writes only its own table.
#[test]
fn every_write_of_an_edit_is_a_commit_of_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let file = project(dir.path(), "p1");
    let target = ProjectDbTarget::from_owner_binding(&file, URI).unwrap();
    let mut client = target.open(Access::ReadWrite, vec![owned_tables::<Notes>()]).unwrap();
    client.execute("test.notes", serde_json::json!({ "kind": "put", "value": "first" })).unwrap();
    client.execute("test.notes", serde_json::json!({ "kind": "put", "value": "second" })).unwrap();
    client.execute("test.notes", serde_json::json!({ "kind": "count" })).unwrap();
    drop(client);
    let commits = qnc_project_journal::read(&rusqlite::Connection::open(&file).unwrap(), 10).unwrap();
    assert_eq!(commits.len(), 2, "two writes, the read is none");
    assert_eq!((commits[0].module.as_str(), commits[0].action.as_str()), ("test.notes", "put"));
    assert_eq!(commits[0].author, local_author());
    assert_eq!((commits[0].changes[0].before["text"].as_str(), commits[0].changes[0].after["text"].as_str()), (Some("first"), Some("second")));
}

/// Audit S-1: the write of an edit and its commit of the journal are one transaction. A journal that
/// cannot be appended to takes the write back with it (Undo, the history and going back never miss
/// an edit that is written), and the rows the refused write changed never join the next commit (S-9).
#[test]
fn an_edit_whose_commit_cannot_be_kept_is_not_written() {
    let dir = tempfile::tempdir().unwrap();
    let file = project(dir.path(), "p1");
    let target = ProjectDbTarget::from_owner_binding(&file, URI).unwrap();
    let mut client = target.open(Access::ReadWrite, vec![owned_tables::<Notes>()]).unwrap();
    let put = |text: &str| serde_json::json!({ "kind": "put", "value": text });
    client.execute("test.notes", put("first")).unwrap();
    let plain = rusqlite::Connection::open(&file).unwrap();
    plain.pragma_update(None, "journal_mode", "PERSIST").unwrap(); // as the broker keeps it
    plain.execute_batch("CREATE TRIGGER refuse AFTER INSERT ON project_journal BEGIN SELECT RAISE(ABORT, 'journal refused'); END").unwrap();
    assert!(client.execute("test.notes", put("lost")).is_err(), "the edit is refused with its commit");
    let text: String = plain.query_row("SELECT text FROM notes WHERE id = 'n'", [], |row| row.get(0)).unwrap();
    assert_eq!(text, "first", "the refused edit is not written");
    plain.execute_batch("DROP TRIGGER refuse").unwrap();
    client.execute("test.notes", put("third")).unwrap();
    drop(client);
    let commits = qnc_project_journal::read(&plain, 10).unwrap();
    assert_eq!(commits.len(), 2, "the first edit and the third, the refused one left nothing");
    let newest = &commits[0];
    assert_eq!(newest.changes.len(), 1, "only the third edit's row");
    assert_eq!((newest.changes[0].before["text"].as_str(), newest.changes[0].after["text"].as_str()), (Some("first"), Some("third")));
}
