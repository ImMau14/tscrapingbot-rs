// Program entry for handling the /start command

use crate::handlers::utils::{ChatActionKeepAlive, send_reply_or_plain};
use teloxide::{
    prelude::*,
    types::{ChatAction, ThreadId},
};
use tracing::error;

pub async fn start(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Spawn a keep-alive task to show typing action while processing
    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    // Get the current crate version for the greeting message
    let tsbot_version = env!("CARGO_PKG_VERSION");
    let message = format!(
        "Hello! I'm TScrapingBot v{tsbot_version}, your Telegram assistant for web scraping and artificial intelligence. Use /help to see my commands."
    );

    keep.shutdown().await;

    if let Err(e) = send_reply_or_plain(&bot, &msg, message, false, false).await {
        error!("Telegram send failed: {e}");
        return Err(e);
    }

    Ok(())
}
