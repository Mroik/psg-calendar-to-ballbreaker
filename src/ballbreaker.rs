use std::time::Duration;

use clokwerk::{AsyncScheduler, Interval, Job};
use google_calendar::Client;
use teloxide::Bot;
use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    time::sleep,
};

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
    config: Config,
}

impl BallBreaker {
    fn new(
        config: Config,
        telegram_token: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        token: &str,
        refresh_token: &str,
    ) -> Self {
        let g_client = Client::new(client_id, client_secret, redirect_uri, token, refresh_token);
        let t_client = Bot::new(telegram_token);

        let (tx, rx) = channel(100);

        BallBreaker {
            g_client,
            t_client,
            rx,
            tx,
            config,
        }
    }

    async fn start(&mut self, time: &str) {
        let tx = self.tx.clone();
        let (tick_killer, mut tick_killer_r) = channel(1);
        let mut scheduler = create_tick_generator(tx, time).await;
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

    // TODO
    async fn mark_as_done(&mut self, calendar_id: &str, event_id: &str) {
        todo!()
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
