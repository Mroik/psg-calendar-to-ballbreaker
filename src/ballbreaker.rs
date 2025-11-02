use google_calendar::Client;
use teloxide::Bot;
use tokio::sync::mpsc::{Receiver, Sender, channel};

enum BBMessage {}

struct BallBreaker {
    g_client: Client,
    t_client: Bot,
    rx: Receiver<BBMessage>,
    tx: Sender<BBMessage>,
}

impl BallBreaker {
    fn new(
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
        }
    }
}
