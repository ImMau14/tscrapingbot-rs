// Ask command handler

use crate::{
    gemini::Gemini,
    handlers::utils::{ChatActionKeepAlive, escape_telegram_code_entities},
    prompts::{GeminiPrompt, Prompt},
};
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::ParseMode,
    types::{ChatAction, ThreadId},
};

pub async fn ask(
    bot: Bot,
    msg: Message,
    text: String,
    gemini: Arc<Gemini>,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Keep-alive typing indicator while we wait for model(s).
    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    // Get prompts struct
    let prompts = GeminiPrompt::new();

    // Obtain response for Gemini
    let res: String = match gemini
        .request()
        .set_model("gemini-2.5-flash")
        .set_temperature(0.0)
        .set_top_p(1.0)
        .set_top_k(1)
        .set_max_output_tokens(2000)
        .set_thinking_budget(1000)
        .set_system_instruction(prompts.get(Prompt::ThinkAndFormat))
        .add_text(&text)
        .send()
        .await
    {
        Ok(response) => escape_telegram_code_entities(&response.formatted(false)),
        Err(e) => {
            // Send error to same chat / thread
            let send_err = if let Some(tid) = thread_id {
                bot.send_message(chat_id, format!("Error: {e}"))
                    .message_thread_id(tid)
                    .await
            } else {
                bot.send_message(chat_id, format!("Error: {e}")).await
            };
            let _ = send_err;
            return Ok(());
        }
    };

    // Stop typing indicator before sending reply.
    keep.shutdown().await;

    // Reply to user
    let send_req = if let Some(tid) = thread_id {
        bot.send_message(chat_id, res)
            .message_thread_id(tid)
            .parse_mode(ParseMode::Html)
    } else {
        bot.send_message(chat_id, res).parse_mode(ParseMode::Html)
    };

    match send_req.await {
        Ok(_) => Ok(()),
        Err(e) => {
            // Try to notify main chat if sending fails
            let _ = bot.send_message(chat_id, e.to_string()).await;
            Err(e)
        }
    }
}
