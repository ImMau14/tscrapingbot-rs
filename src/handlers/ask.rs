use crate::gemini::Gemini;
use std::sync::Arc;
use teloxide::prelude::*;

pub async fn ask(bot: Bot, msg: Message, text: String, gemini: Arc<Gemini>) {
    let _ = bot.send_message(msg.chat.id, gemini.ask(text).await).await;
}
