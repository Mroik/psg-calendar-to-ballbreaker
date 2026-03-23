use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local, NaiveDate};
use clokwerk::{AsyncScheduler, Interval, Job};
use log::info;
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
        format_events_and_send(data_handler, bot)
    });

    async move {
        info!("Scheduler started");
        loop {
            scheduler.run_pending().await;
            sleep(Duration::from_secs(30)).await;
        }
    }
}

pub async fn format_events_and_send(data_handler: Arc<DataHandler>, bot: Bot) {
    let events = data_handler.get_events().await.unwrap();
    if events.is_empty() {
        return;
    }

    let mut message = String::from("These are the pending tasks:\n");
    events.iter().for_each(|(i, event)| {
        message.push_str(&format!("{} - [", i));

        let date = match event.start.date_time.as_ref() {
            Some(v) => DateTime::<Local>::from_str(v).unwrap().date_naive(),
            None => NaiveDate::from_str(event.start.date.as_ref().unwrap()).unwrap(),
        };
        message.push_str(&date.format("%d/%m/%y").to_string());

        let current = DateTime::<Local>::from(SystemTime::now()).date_naive();
        if (date - current).num_days() < 2 {
            message.push_str("⚠️");
        }

        message.push_str("] ");
        message.push_str(&event.summary);
        message.push('\n');
    });

    let keyboard: Vec<Vec<InlineKeyboardButton>> = events
        .iter()
        .map(|(i, event)| {
            let date = match event.start.date_time.as_ref() {
                Some(v) => DateTime::<Local>::from_str(v).unwrap().date_naive(),
                None => NaiveDate::from_str(event.start.date.as_ref().unwrap()).unwrap(),
            }
            .format("%d/%m/%y")
            .to_string();

            let but = InlineKeyboardButton::callback(format!("{} - {}", i, date), format!("{}", i));
            vec![but]
        })
        .collect();

    bot.send_message(Recipient::Id(ChatId(data_handler.chat_id)), message)
        .reply_markup(ReplyMarkup::inline_kb(keyboard))
        .await
        .unwrap();
    info!("Reminder sent");
}
