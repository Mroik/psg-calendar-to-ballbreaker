use anyhow::Result;
use chrono::{Duration, Local};
use gcal_rs::{Event, EventClient, GCalClient, OAuth, OToken};
use log::info;
use rusqlite::{Connection, params_from_iter};
use tokio::sync::Mutex;

/// Handles data retrieval with google's API and persistent data
pub struct DataHandler {
    g_client: Mutex<EventClient>,
    sql_conn: Mutex<Connection>,
    time_window: Duration,
    otoken: Mutex<OToken>,
    pub chat_id: i64,
    calendar_id: String,
    client_id: String,
    client_secret: String,
    refresh: String,
}

impl DataHandler {
    pub async fn new(
        time_window: Duration,
        chat_id: &str,
        calendar_id: &str,
        client_id: &str,
        client_secret: &str,
        db_loc: &str,
    ) -> Result<Self> {
        let sql_conn = Connection::open(db_loc)?;
        create_db(&sql_conn).await?;

        // TODO: Save token somewhere to reuse on restart
        let otoken = OAuth::new(client_id, client_secret, "http://localhost:5000/auth")
            .naive()
            .await?;
        let refresh = otoken.refresh.clone().unwrap();
        let g_client = GCalClient::new(otoken.clone(), None)?.event_client();

        let chat_id = chat_id.parse()?;
        let calendar_id = String::from(calendar_id);

        sql_conn.pragma_update(None, "foreign_keys", "1")?;

        Ok(DataHandler {
            g_client: Mutex::new(g_client),
            sql_conn: Mutex::new(sql_conn),
            time_window,
            otoken: Mutex::new(otoken),
            chat_id,
            calendar_id,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            refresh,
        })
    }

    pub async fn mark_as_done(&self, id: i64) -> Result<()> {
        self.sql_conn
            .lock()
            .await
            .execute("INSERT OR IGNORE INTO done (id) VALUES (?)", [id])?;
        info!("Task {} marked as done", id);
        Ok(())
    }

    pub async fn mark_as_undone(&self, id: i64) -> Result<()> {
        self.sql_conn
            .lock()
            .await
            .execute("DELETE FROM done WHERE id = ?", [id])?;
        info!("Task {} marked as TODO", id);
        Ok(())
    }

    pub async fn get_events(&self) -> Result<Vec<(i64, Event)>> {
        {
            let mut tok = self.otoken.lock().await;
            let mut g_c = self.g_client.lock().await;
            if tok.is_expired() {
                let a = OAuth::new(
                    &self.client_id,
                    &self.client_secret,
                    "http://localhost:5000",
                )
                .exhange_refresh(&self.refresh)
                .await?;
                tok.take_over(a);
                *g_c = GCalClient::new(tok.clone(), None)?.event_client();
                info!("Token refreshed");
            }
        }

        let conn = self.sql_conn.lock().await;
        let mut events = self
            .g_client
            .lock()
            .await
            .list(
                self.calendar_id.clone(),
                Local::now(),
                Local::now() + self.time_window,
            )
            .await?;

        let mut query = String::from("INSERT OR IGNORE INTO events (event_id) VALUES ");
        let placeholders = events
            .iter()
            .map(|_| String::from("(?)"))
            .collect::<Vec<String>>()
            .join(", ");
        query.push_str(&placeholders);

        let params = params_from_iter(events.iter().map(|event| event.id.clone()));
        conn.execute(&query, params)?;
        info!("New events inserted");

        let mut query = String::from(
            "SELECT events.id as id, events.event_id as event_id FROM events
            LEFT JOIN done ON events.id = done.id
            WHERE done.id IS NULL AND events.event_id IN (",
        );
        let a = events
            .iter()
            .map(|_| String::from("?"))
            .collect::<Vec<String>>()
            .join(", ");
        query.push_str(&a);
        query.push(')');

        let params = params_from_iter(events.iter().map(|event| event.id.clone()));
        let mut v = conn.prepare(&query)?;

        let to_keep = v
            .query_map(params, |row| {
                Ok((
                    row.get::<usize, i64>(0).unwrap(),
                    row.get::<usize, String>(1).unwrap(),
                ))
            })?
            .map(|v| v.unwrap())
            .collect::<Vec<(i64, String)>>();
        info!("Events filtered out");

        events.retain(|ev| to_keep.iter().any(|e| ev.id == e.1));
        Ok(events
            .iter()
            .map(|ev| (to_keep.iter().find(|e| e.1 == ev.id).unwrap().0, ev.clone()))
            .collect::<Vec<(i64, Event)>>())
    }
}

async fn create_db(sql_conn: &Connection) -> Result<()> {
    sql_conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id INTEGER NOT NULL,
            UNIQUE (event_id)
        )",
        (),
    )?;
    sql_conn.execute(
        "CREATE TABLE IF NOT EXISTS done (
            id INTEGER PRIMARY KEY,
            FOREIGN KEY (id) REFERENCES events (id)
        )",
        (),
    )?;
    Ok(())
}
