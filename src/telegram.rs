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

use crate::data_handler::DataHandler;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands")]
enum Command {
    #[command(description = "Mark as not done")]
    Todo(i64),
}

pub async fn generate_dispatcher(
    bot: Bot,
    data_handler: Arc<DataHandler>,
) -> Dispatcher<Bot, Error, DefaultKey> {
    let command_data_handler = data_handler.clone();
    let schema = teloxide::dptree::entry()
        .branch(
            Update::filter_callback_query()
                .map(move |_: CallbackQuery| data_handler.clone())
                .endpoint(reply_callback),
        )
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .map(move |_: Message| command_data_handler.clone())
                .branch(case![Command::Todo(i64)].endpoint(undone)),
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
    let Command::Todo(id) = Command::parse(update.text().unwrap(), selferino.username())?;
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
