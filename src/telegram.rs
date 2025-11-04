use std::sync::Arc;

use anyhow::{Error, Result};
use teloxide::{
    Bot,
    dispatching::{DefaultKey, UpdateFilterExt},
    payloads::AnswerCallbackQuerySetters,
    prelude::{Dispatcher, Requester},
    types::{CallbackQuery, Update},
};

use crate::data_handler::DataHandler;

pub async fn generate_dispatcher(
    bot: Bot,
    data_handler: Arc<DataHandler>,
) -> Dispatcher<Bot, Error, DefaultKey> {
    let schema = Update::filter_callback_query()
        .map(move |_: CallbackQuery| data_handler.clone())
        .endpoint(reply_callback);
    Dispatcher::builder(bot, schema).build()
}

// TODO
async fn reply_callback(
    bot: Bot,
    data_handler: Arc<DataHandler>,
    update: CallbackQuery,
) -> Result<()> {
    let data: i64 = update.data.unwrap().parse()?;
    data_handler.mark_as_done(data).await?;
    bot.answer_callback_query(update.id)
        .text(format!("Task with ID {} has been marked as done", data))
        .await?;
    Ok(())
}
