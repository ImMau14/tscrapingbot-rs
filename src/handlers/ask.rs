// Ask command handler

use crate::{gemini::Gemini, handlers::utils::ChatActionKeepAlive};
use std::sync::Arc;
use teloxide::{prelude::*, types::ChatAction};

pub async fn ask(
    bot: Bot,
    msg: Message,
    text: String,
    gemini: Arc<Gemini>,
) -> Result<(), teloxide::RequestError> {
    // Get chat id to reply.
    let chat_id = msg.chat.id;

    // Spawn keep-alive typing indicator.
    let mut keep = ChatActionKeepAlive::spawn(bot.clone(), chat_id, ChatAction::Typing, 4);

    // Build request with the builder API.
    let req = gemini
        .request()
        .set_model("gemini-2.5-flash")
        .set_temperature(0.25)
        .set_max_output_tokens(2048)
        .set_include_thoughts(true)
        .set_thinking_budget(8192)
        .add_text(&text);

    // Send the request and get typed GeminiResult.
    let res = req.send().await;

    // Stop typing indicator before sending reply.
    keep.shutdown().await;

    // Build reply deciding whether to show thoughts or not.
    match res {
        Ok(g) => {
            let reply = g.formatted(false);
            bot.send_message(chat_id, reply).await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Error contacting model: {}", e))
                .await?;
        }
    }

    Ok(())
}
