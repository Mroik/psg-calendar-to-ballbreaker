use std::{sync::Arc, time::Duration};

use anyhow::{Error, Result};
use log::info;
use teloxide::{
    Bot,
    dispatching::{DefaultKey, HandlerExt, UpdateFilterExt},
    dptree::case,
    payloads::AnswerCallbackQuerySetters,
    prelude::{Dispatcher, Requester},
    types::{CallbackQuery, Message, Update},
    utils::command::BotCommands,
};
use tokio::{spawn, time::sleep};

use crate::{data_handler::DataHandler, scheduled::format_events_and_send};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands")]
enum Command {
    #[command(description = "Mark as not done")]
    Todo(i64),
    #[command(description = "Force data querying")]
    Force,
}

pub async fn generate_dispatcher(
    bot: Bot,
    data_handler: Arc<DataHandler>,
) -> Dispatcher<Bot, Error, DefaultKey> {
    let todo_data_handler = data_handler.clone();
    let force_data_handler = data_handler.clone();
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
        );
    info!("About to deploy dispatcher");
    Dispatcher::builder(bot, schema).build()
}

async fn reply_callback(
    bot: Bot,
    data_handler: Arc<DataHandler>,
    update: CallbackQuery,
) -> Result<()> {
    let data: i64 = update.data.unwrap().parse()?;
    data_handler.mark_as_done(data).await?;
    bot.answer_callback_query(update.id)
        .text(format!("Task with ID {} has been marked as done", data))
        .show_alert(true)
        .await?;
    Ok(())
}

async fn undone(bot: Bot, data_handler: Arc<DataHandler>, update: Message) -> Result<()> {
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
    bot.delete_message(update.chat.id, update.id).await?;
    format_events_and_send(data_handler, bot).await;
    Ok(())
}
