use std::time::Duration;

use anyhow::Result;
use clokwerk::{AsyncScheduler, Interval, Job};
use google_calendar::Client;
use log::error;
use rusqlite::Connection;
use teloxide::Bot;
use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    time::sleep,
};

use crate::sql::insert_done_record;

enum BBMessage {
    Stop,
    Tick,
    MarkAsDone {
        calendar_id: String,
        event_id: String,
    },
}

struct Config {
    time_window: Duration,
    chat_id: String,
}

struct BallBreaker {
    g_client: Client,
    t_client: Bot,
    rx: Receiver<BBMessage>,
    tx: Sender<BBMessage>,
    sql_conn: Connection,
    config: Config,
}

impl BallBreaker {
    // TODO: Create DB if not exist
    fn new(
        config: Config,
        telegram_token: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        token: &str,
        refresh_token: &str,
        db_loc: &str,
    ) -> Result<Self> {
        let g_client = Client::new(client_id, client_secret, redirect_uri, token, refresh_token);
        let t_client = Bot::new(telegram_token);

        let (tx, rx) = channel(100);
        let sql_conn = Connection::open(db_loc)?;

        Ok(BallBreaker {
            g_client,
            t_client,
            rx,
            tx,
            config,
            sql_conn,
        })
    }

    async fn start(&mut self, time: &str) {
        let (tick_killer, mut tick_killer_r) = channel(1);
        let mut scheduler = create_tick_generator(self.tx.clone(), time).await;
        tokio::join!(self.run(tick_killer), async move {
            loop {
                tokio::select! {
                    _ = scheduler.run_pending() => {}
                    _ = tick_killer_r.recv() => { break; }
                };
                sleep(Duration::from_secs(30)).await;
            }
        });
    }

    async fn run(&mut self, tick_killer: Sender<()>) {
        loop {
            match self.rx.recv().await {
                None | Some(BBMessage::Stop) => {
                    self.save_to_disk().await;
                    tick_killer.send(()).await.unwrap();
                    return;
                }
                Some(BBMessage::MarkAsDone {
                    calendar_id,
                    event_id,
                }) => self.mark_as_done(&calendar_id, &event_id).await,
                Some(BBMessage::Tick) => self.send_reminders().await,
            }
        }
    }

    // TODO
    async fn send_reminders(&mut self) {
        todo!()
    }

    // TODO
    async fn save_to_disk(&mut self) {
        todo!()
    }

    async fn mark_as_done(&mut self, calendar_id: &str, event_id: &str) {
        match insert_done_record(&self.sql_conn, event_id, calendar_id) {
            Ok(_) => (),
            Err(e) => error!("Error on `done insert`: {e}"),
        };
    }
}

async fn create_tick_generator(tx: Sender<BBMessage>, time: &str) -> AsyncScheduler {
    let mut scheduler = AsyncScheduler::new();
    scheduler.every(Interval::Seconds(1)).at(time).run(move || {
        let tx = tx.clone();
        async move {
            let _ = tx.send(BBMessage::Tick).await;
        }
    });
    scheduler
}
