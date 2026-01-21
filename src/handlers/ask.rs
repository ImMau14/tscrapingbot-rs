// /ask command handler that builds context, preprocesses images, and routes prompts through LLMs.

use crate::{
    config::Models,
    handlers::{
        types::MessageRow,
        utils::{
            ChatActionKeepAlive, escape_telegram_code_entities, extract_user_info,
            llm::{analyze_image, message_has_photo},
            send_reply_or_plain,
        },
    },
    prompts::{AiPrompt, Prompt},
};
use groqai::{ChatMessage, GroqClient, MessageContent, Role};
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatAction, ThreadId},
};
use tracing::error;

// /ask command handler that builds context, preprocesses images, and routes prompts through LLMs.
pub async fn ask(
    bot: Bot,
    msg: Message,
    text: String,
    pool: PgPool,
    groq: GroqClient,
    models: Models,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Keep Telegram "typing" action alive during long processing.
    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    if text.trim().is_empty() {
        keep.shutdown().await;
        send_reply_or_plain(
            &bot,
            &msg,
            "I can't reply to an empty message. Use /ask <query>.",
            false,
            false,
        )
        .await?;
        return Ok(());
    }

    // Prompt helper to access predefined system prompts.
    let prompts = AiPrompt::new();

    let (user_id, user_lang, msg_chat_id) = match extract_user_info(&msg) {
        Ok(v) => v,
        Err(err_msg) => {
            // User-facing error, stop typing indicator, return
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, err_msg, false, false).await?;
            return Ok(());
        }
    };

    // Load recent messages using your stored procedure.
    let history_limit: i32 = 30;
    let mut messages: Vec<MessageRow> = match sqlx::query_as!(
        MessageRow,
        "SELECT content, ia_response FROM get_recent_messages($1, $2, $3, $4)",
        user_lang,
        user_id,
        msg_chat_id,
        history_limit,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!("Query failed: {e}");
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Database error.", false, false).await?;
            return Ok(());
        }
    };
    messages.reverse();

    let image_section = if message_has_photo(&msg) {
        analyze_image(
            &bot,
            &msg,
            &text,
            &prompts.get(Prompt::Vision),
            messages.clone(),
            &groq,
            &models.clone().vision,
        )
        .await
    } else {
        String::new()
    };

    let main_model = &models.thinking;

    // Build conversation messages: system prompt, previous turns (user -> assistant), then current user message.
    let system_prompt = prompts.get(Prompt::ThinkAndFormat);

    let mut convo: Vec<ChatMessage> = Vec::new();
    convo.push(ChatMessage::new_text(Role::System, system_prompt));

    // Append historical turns (if any). For each saved row: user content then assistant response.
    for row in &messages {
        if let Some(ref user_content) = row.content {
            convo.push(ChatMessage::new_text(Role::User, user_content.clone()));
        }
        if let Some(ref assistant_content) = row.ia_response {
            convo.push(ChatMessage::new_text(
                Role::Assistant,
                assistant_content.clone(),
            ));
        }
    }

    // Current user message: include image_section if present.
    let current_user_msg = if image_section.is_empty() {
        format!(
            "Main lang is \"{user_lang}\":\n\nOriginal prompt: {}\n",
            text
        )
    } else {
        format!(
            "Main lang is \"{user_lang}\":\n\nOriginal prompt: {}\n\nImage analysis:\n{}\n",
            text, image_section
        )
    };
    convo.push(ChatMessage::new_text(Role::User, current_user_msg));

    // Call the main model directly with the conversation (no intermediate reasoning step).
    let resp = match groq
        .chat(main_model)
        .messages(convo)
        .max_completion_tokens(3000)
        .temperature(0.0)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Model error
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, format!("Error: {e}."), false, false).await?;
            return Ok(());
        }
    };

    // Extract textual content (same logic you had in helpers).
    let raw_answer = if let MessageContent::Text(text) = &resp.choices[0].message.content {
        text.trim().to_string()
    } else {
        String::new()
    };

    // Escape for Telegram HTML before sending and saving.
    let final_answer = escape_telegram_code_entities(&raw_answer);

    keep.shutdown().await;

    let send_req = send_reply_or_plain(&bot, &msg, final_answer.clone(), false, true);

    if let Err(e) = send_req.await {
        error!("Telegram send failed: {e} — no DB transaction to roll back.");
        return Ok(());
    }

    if let Err(e) = sqlx::query!(
        r#"
        INSERT INTO messages (user_telegram_id, chat_telegram_id, content, ia_response)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        msg_chat_id,
        text,
        final_answer,
    )
    .execute(&pool)
    .await
    {
        error!("Insert failed: {e}");
        send_reply_or_plain(
            &bot,
            &msg,
            "Database error (couldn't save message).",
            false,
            false,
        )
        .await?;
    }

    Ok(())
}
