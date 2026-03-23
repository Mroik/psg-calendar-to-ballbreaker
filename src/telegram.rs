use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use log::info;
use teloxide::{
    Bot,
    dispatching::{DefaultKey, HandlerExt, UpdateFilterExt, dialogue::GetChatId},
    dptree::case,
    payloads::AnswerCallbackQuerySetters,
    prelude::{Dispatcher, Requester},
    types::{CallbackQuery, InputFile, Message, Update},
    utils::command::BotCommands,
};
use tokio::{spawn, time::sleep};

use crate::{data_handler::DataHandler, scheduled::format_events_and_send};

const VCAL_PRODID: &str = "PSG Calendar To Ballbreaker";
const PSG_EMAIL: &str = "polimisocialgames@gmail.com";
const ORG_NAME: &str = "Polimi Social Games";

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands")]
enum Command {
    #[command(description = "Mark as not done")]
    Todo(i64),
    #[command(description = "Force data querying")]
    Force,
    #[command(description = "Generate ics")]
    Invite(i64),
}

pub async fn generate_dispatcher(
    bot: Bot,
    data_handler: Arc<DataHandler>,
) -> Dispatcher<Bot, Error, DefaultKey> {
    let todo_data_handler = data_handler.clone();
    let force_data_handler = data_handler.clone();
    let generate_invite_data_handler = data_handler.clone();
    let schema = teloxide::dptree::entry()
        .branch(
            Update::filter_callback_query()
                .map(move |_: CallbackQuery| data_handler.clone())
                .endpoint(reply_callback),
        )
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .map(move |_: Message| todo_data_handler.clone())
                .branch(case![Command::Todo(i64)].endpoint(undone)),
        )
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .map(move |_: Message| force_data_handler.clone())
                .branch(case![Command::Force].endpoint(force)),
        )
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .map(move |_: Message| generate_invite_data_handler.clone())
                .branch(case![Command::Invite(i64)].endpoint(generate_invite)),
        );
    info!("About to deploy dispatcher");
    Dispatcher::builder(bot, schema).build()
}

async fn reply_callback(
    bot: Bot,
    data_handler: Arc<DataHandler>,
    update: CallbackQuery,
) -> Result<()> {
    match update.chat_id() {
        Some(c_i) if c_i.0 != data_handler.chat_id => {
            return Ok(());
        }
        Some(_) => (),
        None => return Ok(()),
    }

    let data: i64 = update.data.unwrap().parse()?;
    data_handler.mark_as_done(data).await?;
    bot.answer_callback_query(update.id)
        .text(format!("Task with ID {} has been marked as done", data))
        .show_alert(true)
        .await?;
    Ok(())
}

async fn undone(bot: Bot, data_handler: Arc<DataHandler>, update: Message) -> Result<()> {
    if update.chat.id.0 != data_handler.chat_id {
        return Ok(());
    }

    let selferino = bot.get_me().await?;
    let id = match Command::parse(update.text().unwrap(), selferino.username())? {
        Command::Todo(id) => id,
        _ => unreachable!(),
    };
    data_handler.mark_as_undone(id).await?;
    let chat_id = update.chat.id;
    let to_delete = bot
        .send_message(
            chat_id,
            format!("Task with ID {} has been marked as todo", id),
        )
        .await?;

    bot.delete_message(chat_id, update.id).await?;

    spawn(async move {
        sleep(Duration::from_mins(3)).await;
        bot.clone()
            .delete_message(chat_id, to_delete.id)
            .await
            .unwrap();
        info!("Deleted info message");
    });

    Ok(())
}

async fn force(bot: Bot, data_handler: Arc<DataHandler>, update: Message) -> Result<()> {
    if update.chat.id.0 != data_handler.chat_id {
        return Ok(());
    }

    bot.delete_message(update.chat.id, update.id).await?;
    format_events_and_send(data_handler, bot).await;
    Ok(())
}

fn datetime2ical(d: &DateTime<Utc>) -> String {
    d.format("%Y%m%dT%H%M%SZ").to_string()
}

async fn generate_invite(bot: Bot, data_handler: Arc<DataHandler>, update: Message) -> Result<()> {
    if update.chat.id.0 != data_handler.chat_id {
        return Ok(());
    }

    let id = match Command::parse(update.text().unwrap(), bot.get_me().await?.username())? {
        Command::Invite(id) => id,
        _ => unreachable!(),
    };

    bot.delete_message(update.chat.id, update.id).await?;

    let ev = match data_handler
        .get_events()
        .await?
        .iter()
        .find(|e| e.0 == id)
        .cloned()
    {
        Some(ev) => ev.1,
        None => {
            let to_delete = bot
                .send_message(update.chat.id, format!("Task with ID {} not found", id))
                .await?;
            spawn(async move {
                sleep(Duration::from_mins(3)).await;
                bot.clone()
                    .delete_message(update.chat.id, to_delete.id)
                    .await
                    .unwrap();
                info!("Deleted info message");
            });
            return Ok(());
        }
    };

    let dt = datetime2ical(
        &DateTime::from_timestamp_secs(UNIX_EPOCH.elapsed().unwrap().as_secs() as i64).unwrap(),
    );
    let start = datetime2ical(&DateTime::from_str(&ev.start.date_time.unwrap())?);
    let end = datetime2ical(&DateTime::from_str(&ev.end.date_time.unwrap())?);

    let mut data = String::new();
    data.push_str("BEGIN:VCALENDAR\r\n");
    data.push_str("VERSION:2.0\r\n");
    data.push_str(&format!("PRODID:{}\r\n", VCAL_PRODID));
    data.push_str("BEGIN:VEVENT\r\n");
    data.push_str(&format!("UID:{}\r\n", ev.id));
    data.push_str(&format!(
        "ORGANIZER;CN={};SENT-BY=\"mailto:{}\":mailto:{}\r\n",
        ORG_NAME, PSG_EMAIL, PSG_EMAIL
    ));
    data.push_str(&format!("DTSTAMP:{}\r\n", dt));
    data.push_str(&format!("DTSTART:{}\r\n", start));
    data.push_str(&format!("DTEND:{}\r\n", end));
    data.push_str(&format!("SUMMARY:{}\r\n", ev.summary));
    data.push_str("END:VEVENT\r\n");
    data.push_str("END:VCALENDAR\r\n");

    bot.send_document(
        update.chat.id,
        InputFile::memory(data).file_name("evento_PSG.ics"),
    )
    .await?;
    Ok(())
}
