// /ask command handler that builds context, preprocesses images, and routes prompts through LLMs.

use crate::{
    handlers::{
        types::MessageRow,
        utils::{
            ChatActionKeepAlive, escape_telegram_code_entities, extract_user_info,
            format_messages_xml,
            llm::{analyze_image, message_has_photo, run_main_model, run_reasoning_step},
        },
    },
    prompts::AiPrompt,
};
use groqai::GroqClient;
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatAction, ParseMode, ThreadId},
};
use tracing::error;

// Handles the /ask command lifecycle: context loading, iamge analysis, LLM calls, and persistence.
pub async fn ask(
    bot: Bot,
    msg: Message,
    text: String,
    pool: PgPool,
    groq: GroqClient,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Keep Telegram "typing" action alive during long processing.
    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    // Prompt helper to access predefined system prompts.
    let prompts = AiPrompt::new();

    let (user_id, user_lang, msg_chat_id) = match extract_user_info(&msg) {
        Ok(v) => v,
        Err(err_msg) => {
            // User-facing error, stop typing indicator, return
            keep.shutdown().await;
            bot.send_message(chat_id, err_msg).await?;
            return Ok(());
        }
    };

    // Start database transaction.
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("DB transaction error: {e}");
            keep.shutdown().await;
            bot.send_message(chat_id, "Internal database error.")
                .await?;
            return Ok(());
        }
    };

    // Load recent messages using your stored procedure.
    let history_limit: i32 = 30;
    let messages: Vec<MessageRow> = match sqlx::query_as!(
        MessageRow,
        "SELECT content, ia_response FROM get_recent_messages($1, $2, $3, $4)",
        user_lang,
        user_id,
        msg_chat_id,
        history_limit,
    )
    .fetch_all(&mut *tx)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!("Query failed: {e}");
            let _ = tx.rollback().await;
            keep.shutdown().await;
            bot.send_message(chat_id, "Database error.").await?;
            return Ok(());
        }
    };

    let history = format_messages_xml(&messages, 0, false);

    let image_section = if message_has_photo(&msg) {
        analyze_image(
            &bot,
            &msg,
            &format!("{text}\n\nHistory:\n\n{history}"),
            &groq,
        )
        .await
    } else {
        String::new()
    };

    let base_prompt = format!("{text}\n\n{image_section}History:\n\n{history}");
    let reasoning_model = "openai/gpt-oss-20b";
    let main_model = "openai/gpt-oss-120b";

    let refined = match run_reasoning_step(&groq, &prompts, &base_prompt, reasoning_model).await {
        Some(v) => v,
        None => {
            // Fatal preprocessing error: rollback and notify
            let _ = tx.rollback().await;
            keep.shutdown().await;
            bot.send_message(chat_id, "Error during preprocessing.")
                .await?;
            return Ok(());
        }
    };

    let prompt_for_main = format!(
        "Main lang is \"{user_lang}\":\n\nOriginal prompt: {}\n\nResource for you response: {}",
        text, refined
    );

    let raw_answer = match run_main_model(&groq, &prompts, &prompt_for_main, main_model).await {
        Ok(v) => v,
        Err(e) => {
            // Model error: rollback and notify
            let _ = tx.rollback().await;
            keep.shutdown().await;
            bot.send_message(chat_id, format!("Error: {e}")).await?;
            return Ok(());
        }
    };

    // Escape for Telegram HTML before sending and saving.
    let final_answer = escape_telegram_code_entities(&raw_answer);

    keep.shutdown().await;

    let send_req = if let Some(tid) = thread_id {
        bot.send_message(chat_id, final_answer.clone())
            .message_thread_id(tid)
            .parse_mode(ParseMode::Html)
    } else {
        bot.send_message(chat_id, final_answer.clone())
            .parse_mode(ParseMode::Html)
    };

    if let Err(e) = send_req.await {
        error!("Telegram send failed: {e} — rolling back DB transaction.");
        let _ = tx.rollback().await;
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
    .execute(&mut *tx)
    .await
    {
        error!("Insert failed: {e}");
        let _ = tx.rollback().await;
        bot.send_message(chat_id, "Database error (couldn't save message).")
            .await?;
        return Ok(());
    }

    // Commit; log and notify user on failure
    if let Err(e) = tx.commit().await {
        error!("Commit failed: {e}");
        bot.send_message(chat_id, "Error saving data.").await?;
    }

    Ok(())
}
