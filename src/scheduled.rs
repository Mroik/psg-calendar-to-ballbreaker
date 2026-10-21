use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local, NaiveDate};
use clokwerk::{AsyncScheduler, Interval, Job};
use gcal_rs::Event;
use log::{error, info};
use teloxide::{
    Bot,
    prelude::Requester,
    types::{ChatId, Recipient},
};
use tokio::time::sleep;

use crate::data_handler::DataHandler;

pub async fn generate_scheduler(
    bot: Bot,
    data_handler: Arc<DataHandler>,
    time: &str,
) -> impl Future<Output = ()> {
    let mut scheduler = AsyncScheduler::new();

    let dh_executive = data_handler.clone();
    let bot_executive = bot.clone();
    scheduler.every(Interval::Minutes(1)).at(time).run(move || {
        let data_handler = dh_executive.clone();
        let bot = bot_executive.clone();
        let chat_id_executive = data_handler.chat_id_executive;
        format_and_send(data_handler, bot, chat_id_executive, |_| true)
    });

    let dh_abn = data_handler.clone();
    let bot_abn = bot.clone();
    scheduler.every(Interval::Minutes(1)).at(time).run(move || {
        let data_handler = dh_abn.clone();
        let bot = bot_abn.clone();
        let chat_id_abn = data_handler.chat_id_abn;
        format_and_send(data_handler, bot, chat_id_abn, |e| {
            let current = DateTime::<Local>::from(SystemTime::now()).date_naive();
            let date = match e.start.date_time.as_ref() {
                Some(v) => DateTime::<Local>::from_str(v).unwrap().date_naive(),
                None => NaiveDate::from_str(e.start.date.as_ref().unwrap()).unwrap(),
            };
            e.summary.to_lowercase().contains("social games") && (date - current).num_days() <= 1
        })
    });

    async move {
        info!("Scheduler started");
        loop {
            scheduler.run_pending().await;
            sleep(Duration::from_secs(30)).await;
        }
    }
}

async fn format_events(events: &[(i64, Event)]) -> String {
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

    message
}

pub async fn format_and_send<F>(data_handler: Arc<DataHandler>, bot: Bot, chat_id: i64, filter: F)
where
    F: Copy + FnMut(&Event) -> bool,
{
    let events;
    let mut attempts = 3;
    loop {
        attempts -= 1;
        match data_handler.get_events(filter).await {
            Ok(e) => {
                events = e;
                break;
            }
            Err(e) => {
                error!("{}", e);
                if attempts == 0 {
                    return;
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        }
    }
    if events.is_empty() {
        return;
    }

    let message = format_events(&events).await;
    bot.send_message(Recipient::Id(ChatId(chat_id)), message)
        .await
        .unwrap();
    info!("Reminder sent");
}
