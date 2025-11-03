use std::time::Duration;

use anyhow::Result;
use chrono::{Local, TimeDelta};
use gcal_rs::{Event, EventClient, GCalClient, OAuth};
use rusqlite::Connection;
use tokio::sync::Mutex;

/// Handles data retrieval with google's API and persistent data
struct DataHandler {
    g_client: EventClient,
    sql_conn: Mutex<Connection>,
    time_window: Duration,
    chat_id: String,
    calendar_id: String,
}

impl DataHandler {
    async fn new(
        time_window: Duration,
        chat_id: &str,
        calendar_id: &str,
        client_id: &str,
        client_secret: &str,
        db_loc: &str,
    ) -> Result<Self> {
        let sql_conn = Connection::open(db_loc)?;
        create_db(&sql_conn).await?;

        let token = OAuth::new(client_id, client_secret, "http://localhost:5000/auth")
            .naive()
            .await?;
        let (_, g_client) = GCalClient::new(token, None)?.clients();
        let chat_id = String::from(chat_id);
        let calendar_id = String::from(calendar_id);

        Ok(DataHandler {
            g_client,
            sql_conn: Mutex::new(sql_conn),
            time_window,
            chat_id,
            calendar_id,
        })
    }

    async fn mark_as_done(&self, event_id: &str) -> Result<()> {
        self.sql_conn.lock().await.execute(
            "INSERT OR IGNORE INTO done (event_id, calendar_id, timestamp) VALUES (?, ?, ?)",
            [
                event_id,
                &self.calendar_id,
                &std::time::UNIX_EPOCH.elapsed()?.as_secs().to_string(),
            ],
        )?;
        Ok(())
    }

    async fn get_events(&self) -> Result<Vec<Event>> {
        Ok(self
            .g_client
            .list(
                self.calendar_id.clone(),
                Local::now(),
                Local::now()
                    .checked_add_signed(TimeDelta::weeks(2))
                    .unwrap(),
            )
            .await?)
    }
}

async fn create_db(sql_conn: &Connection) -> Result<()> {
    sql_conn.execute(
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
