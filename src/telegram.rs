use anyhow::Error;
use teloxide::{
    Bot,
    dispatching::{DefaultKey, UpdateFilterExt},
    prelude::Dispatcher,
    types::Update,
};

pub async fn generate_dispatcher(bot: Bot) -> Dispatcher<Bot, Error, DefaultKey> {
    let schema = Update::filter_callback_query().endpoint(reply_callback);
    Dispatcher::builder(bot, schema).build()
}

// TODO
async fn reply_callback(bot: Bot, update: Update) -> anyhow::Result<()> {
    todo!()
}
