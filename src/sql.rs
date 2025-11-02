use anyhow::Result;
use rusqlite::Connection;

fn create_db(db_loc: &str) -> Result<()> {
    let conn = Connection::open(db_loc)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS done (
            event_id INTEGER NOT NULL,
            calendar_id INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            PRIMARY KEY (event_id, calendar_id)
        )",
        (),
    )?;
    Ok(())
}

pub fn insert_done_record(conn: &Connection, event_id: &str, calendar_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO done (event_id, calendar_id, timestamp) VALUES (?, ?, ?)",
        [
            event_id,
            calendar_id,
            &std::time::UNIX_EPOCH.elapsed()?.as_secs().to_string(),
        ],
    )?;
    Ok(())
}
