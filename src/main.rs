use std::{env, sync::Arc};

use anyhow::Result;
use chrono::Duration;
use teloxide::Bot;

use crate::{
    data_handler::DataHandler, scheduled::generate_scheduler, telegram::generate_dispatcher,
};

mod data_handler;
mod scheduled;
mod telegram;

#[tokio::main]
async fn main() -> Result<()> {
    let time_window = Duration::days(
        env::var("TIME_WINDOW")
            .expect("TIME_WINDOW missing")
            .parse()?,
    );
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID missing");
    let calendar_id = env::var("CALENDAR_ID").expect("CALENDAR_ID missing");
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID missing");
    let client_secret = env::var("CLIENT_SECRET").expect("CLIENT_SECRET missing");
    let data_handler = Arc::new(
        DataHandler::new(
            time_window,
            &chat_id,
            &calendar_id,
            &client_id,
            &client_secret,
            "database.sqlite3",
        )
        .await?,
    );

    let scheduled_time = env::var("SCHEDULED_TIME").expect("SCHEDULED_TIME missing");
    let telegram_token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN missing");
    let bot = Bot::new(telegram_token);

    let scheduler = generate_scheduler(bot.clone(), data_handler, &scheduled_time).await;
    let mut dispatcher = generate_dispatcher(bot).await;

    tokio::join!(scheduler, dispatcher.dispatch());

    Ok(())
}
