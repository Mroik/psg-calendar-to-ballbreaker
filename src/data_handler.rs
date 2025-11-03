use std::time::Duration;

use anyhow::Result;
use google_calendar::Client;
use rusqlite::Connection;

/// Handles data retrieval with google's API and persistent data
struct DataHandler {
    g_client: Client,
    sql_conn: Connection,
    time_window: Duration,
    chat_id: String,
}

impl DataHandler {
    // TODO: Create DB if not exist
    fn new(
        time_window: Duration,
        chat_id: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        token: &str,
        refresh_token: &str,
        db_loc: &str,
    ) -> Result<Self> {
        let g_client = Client::new(client_id, client_secret, redirect_uri, token, refresh_token);
        let sql_conn = Connection::open(db_loc)?;
        let chat_id = String::from(chat_id);

        Ok(DataHandler {
            g_client,
            sql_conn,
            time_window,
            chat_id,
        })
    }

    async fn mark_as_done(&mut self, calendar_id: &str, event_id: &str) -> Result<()> {
        self.sql_conn.execute(
            "INSERT OR IGNORE INTO done (event_id, calendar_id, timestamp) VALUES (?, ?, ?)",
            [
                event_id,
                calendar_id,
                &std::time::UNIX_EPOCH.elapsed()?.as_secs().to_string(),
            ],
        )?;
        Ok(())
    }
}
