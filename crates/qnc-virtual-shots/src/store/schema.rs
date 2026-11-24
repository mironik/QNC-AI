use super::*;

pub(crate) fn ensure_virtual_shots_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS virtual_shot_numbers (
            clip_id TEXT NOT NULL,
            class TEXT NOT NULL,
            last INTEGER NOT NULL,
            PRIMARY KEY (clip_id, class)
        );
        CREATE TABLE IF NOT EXISTS virtual_shot_parts (
            shot_id TEXT NOT NULL,
            part_index INTEGER NOT NULL,
            clip_id TEXT NOT NULL,
            in_frame INTEGER NOT NULL,
            out_frame INTEGER NOT NULL,
            PRIMARY KEY (shot_id, part_index)
        );
        CREATE TABLE IF NOT EXISTS virtual_shots (
            shot_id TEXT PRIMARY KEY,
            clip_id TEXT NOT NULL,
            class TEXT NOT NULL CHECK (class IN ('source', 'short', 'b_roll')),
            in_frame INTEGER NOT NULL,
            out_frame INTEGER NOT NULL,
            source_shot_id TEXT,
            name TEXT NOT NULL,
            created_at_utc INTEGER NOT NULL,
            in_still_uri TEXT,
            out_still_uri TEXT,
            still_status TEXT NOT NULL DEFAULT 'pending'
                CHECK (still_status IN ('pending', 'ready', 'failed')),
            still_error TEXT
        );",
    )
    .map_err(err)?;
    add_virtual_column_if_missing(conn, "in_still_uri", "TEXT")?;
    add_virtual_column_if_missing(conn, "out_still_uri", "TEXT")?;
    add_virtual_column_if_missing(
        conn,
        "still_status",
        "TEXT NOT NULL DEFAULT 'pending' CHECK (still_status IN ('pending', 'ready', 'failed'))",
    )?;
    add_virtual_column_if_missing(conn, "still_error", "TEXT")?;
    // A B-roll shot is kept once its cover is written (third audit 2026-10-17).
    add_virtual_column_if_missing(conn, "cover_pending", "INTEGER NOT NULL DEFAULT 0")?;
    // The caption of a statement (user 2026-11-19): who speaks and their role.
    add_virtual_column_if_missing(conn, "caption_name", "TEXT NOT NULL DEFAULT ''")?;
    add_virtual_column_if_missing(conn, "caption_role", "TEXT NOT NULL DEFAULT ''")?;
    // IN and OUT as the camera's timecode (user 2026-11-20: "time kod je jedino mjerilo").
    add_virtual_column_if_missing(conn, "tc_in", "TEXT NOT NULL DEFAULT ''")?;
    add_virtual_column_if_missing(conn, "tc_out", "TEXT NOT NULL DEFAULT ''")?;
    // How each part joins the one before it (user 2026-11-23: "rez se može napraviti SMOOTH, BLIC, CAT").
    add_column_if_missing(conn, "virtual_shot_parts", "join_kind", "TEXT NOT NULL DEFAULT 'cut'")?;
    conn.execute_batch(
        "DROP VIEW IF EXISTS public_short_clips;
        CREATE VIEW public_short_clips AS
        SELECT shot_id, clip_id, in_frame, out_frame, source_shot_id, name, created_at_utc,
               in_still_uri, out_still_uri, still_status, still_error, caption_name, caption_role, tc_in, tc_out
        FROM virtual_shots
        WHERE class = 'short';
        DROP VIEW IF EXISTS public_b_roll_clips;
        CREATE VIEW public_b_roll_clips AS
        SELECT shot_id, clip_id, in_frame, out_frame, source_shot_id, name, created_at_utc,
               in_still_uri, out_still_uri, still_status, still_error
        FROM virtual_shots
        WHERE class = 'b_roll' AND cover_pending = 0;",
    )
    .map_err(err)
}

pub(crate) fn add_virtual_column_if_missing(conn: &Connection, column: &str, definition: &str) -> Result<()> {
    add_column_if_missing(conn, "virtual_shots", column, definition)
}

pub(crate) fn add_column_if_missing(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(err)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    if columns.iter().any(|existing| existing == column) {
        return Ok(());
    }
    conn.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}"
    ))
    .map_err(err)
}
