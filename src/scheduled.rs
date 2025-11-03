use std::{str::FromStr, sync::Arc, time::Duration};

use chrono::{DateTime, Local, NaiveDate};
use clokwerk::{AsyncScheduler, Interval, Job};
use teloxide::{
    Bot,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{ChatId, InlineKeyboardButton, Recipient, ReplyMarkup},
};
use tokio::time::sleep;

use crate::data_handler::DataHandler;

pub async fn generate_scheduler(
    bot: Bot,
    data_handler: Arc<DataHandler>,
    time: &str,
) -> impl Future<Output = ()> {
    let mut scheduler = AsyncScheduler::new();
    scheduler.every(Interval::Minutes(1)).at(time).run(move || {
        let data_handler = data_handler.clone();
        let bot = bot.clone();
        async move {
            let events = data_handler.get_events().await.unwrap();
            let parts = events
                .iter()
                .enumerate()
                .map(|(i, event)| {
                    let mut s = format!("{} - [", i + 1);

                    let date = match event.start.date_time.as_ref() {
                        Some(v) => DateTime::<Local>::from_str(v).unwrap().date_naive(),
                        None => NaiveDate::from_str(event.start.date.as_ref().unwrap()).unwrap(),
                    }
                    .format("%d/%m/%y")
                    .to_string();

                    s.push_str(&date);
                    s.push_str("] ");
                    s.push_str(&event.summary);
                    s
                })
                .collect::<Vec<String>>()
                .join("\n");
            let mut message = String::from("These are the pending tasks:\n");
            message.push_str(&parts);

            let keyboard: Vec<Vec<InlineKeyboardButton>> = events
                .iter()
                .enumerate()
                .map(|(i, event)| {
                    let date = match event.start.date_time.as_ref() {
                        Some(v) => DateTime::<Local>::from_str(v).unwrap().date_naive(),
                        None => NaiveDate::from_str(event.start.date.as_ref().unwrap()).unwrap(),
                    }
                    .format("%d/%m/%y")
                    .to_string();

                    // TODO
                    let but =
                        InlineKeyboardButton::callback(format!("{} - {}", i + 1, date), "TODO");
                    vec![but]
                })
                .collect();

            bot.send_message(Recipient::Id(ChatId(data_handler.chat_id)), message)
                .reply_markup(ReplyMarkup::inline_kb(keyboard))
                .await
                .unwrap();
        }
    });

    async move {
        loop {
            scheduler.run_pending().await;
            sleep(Duration::from_secs(30)).await;
        }
    }
}
