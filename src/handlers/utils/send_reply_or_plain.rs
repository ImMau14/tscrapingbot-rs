// Sends a reply to a message, handling thread and HTML parsing options

use teloxide::{
    prelude::*,
    requests::Requester,
    types::{ParseMode, ReplyParameters, ThreadId},
};

pub async fn send_reply_or_plain(
    bot: &Bot,
    msg: &Message,
    text: impl Into<String>,
    allow_sending_without_reply: bool,
    parse_html: bool,
) -> Result<Message, teloxide::RequestError> {
    // Extract chat and optional thread identifiers
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Determine if the chat behaves like a group (has a title)
    let is_group_like = msg.chat.title().is_some();

    if is_group_like {
        // Build reply parameters, optionally allowing sending without a reply
        let params = if allow_sending_without_reply {
            ReplyParameters::new(msg.id).allow_sending_without_reply()
        } else {
            ReplyParameters::new(msg.id)
        };

        // Start building the message request with reply parameters
        let mut req = bot
            .send_message(chat_id, text.into())
            .reply_parameters(params);
        // Apply HTML parse mode if requested
        if parse_html {
            req = req.parse_mode(ParseMode::Html);
        }
        // Attach thread ID if present
        let req = if let Some(tid) = thread_id {
            req.message_thread_id(tid)
        } else {
            req
        };

        // Send the request
        req.await
    } else {
        // Build a simple message request without reply parameters
        let mut req = bot.send_message(chat_id, text.into());
        // Apply HTML parse mode if requested
        if parse_html {
            req = req.parse_mode(ParseMode::Html);
        }
        // Send the request
        req.await
    }
}
