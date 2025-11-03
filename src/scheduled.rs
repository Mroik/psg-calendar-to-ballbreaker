use std::{sync::Arc, time::Duration};

use clokwerk::{AsyncScheduler, Interval, Job};
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
    scheduler.every(Interval::Minutes(1)).at(time).run(move || {
        let data_handler = data_handler.clone();
        let bot = bot.clone();
        async move {
            let events = data_handler.get_events().await.unwrap();
            // TODO
            let message = "";
            bot.send_message(Recipient::Id(ChatId(0)), message)
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
