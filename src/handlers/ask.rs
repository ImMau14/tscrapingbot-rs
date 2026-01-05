// /ask command handler that builds context, preprocesses images, and routes prompts through LLMs.

use crate::{
    handlers::{
        types::MessageRow,
        utils::{
            ChatActionKeepAlive, analyze_image, escape_telegram_code_entities, format_messages_xml,
            message_has_photo,
        },
    },
    prompts::{AiPrompt, Prompt},
};
use groqai::{ChatMessage, GroqClient, MessageContent, Role};
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatAction, ParseMode, ThreadId},
};
use tracing::error;

// Handles the /ask command lifecycle: context loading, image analysis, LLM calls, and persistence.
pub async fn ask(
    bot: Bot,
    msg: Message,
    text: String,
    pool: PgPool,
    groq: GroqClient,
) -> Result<(), teloxide::RequestError> {
    // Extract chat and thread identifiers.
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Keep Telegram "typing" action alive during long processing.
    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    // Prompt helper to access predefined system prompts.
    let prompts = AiPrompt::new();

    // Validate message author.
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => {
            keep.shutdown().await;
            bot.send_message(chat_id, "The user could not be identified.")
                .await?;
            return Ok(());
        }
    };

    // Normalize identifiers and user metadata.
    let user_id: i64 = user.id.0 as i64;
    let user_lang: &str = user.language_code.as_deref().unwrap_or("en");
    let msg_chat_id: i64 = thread_id.map(|tid| tid.0.0 as i64).unwrap_or(chat_id.0);
    let history_limit: i32 = 30;

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

    // Load recent message history, creating language/user/chat records if needed.
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

    // Format message history into XML-like structure for prompting.
    let history = format_messages_xml(&messages, 0, false);

    // ---------------------------
    // Image handling
    // ---------------------------

    // Analyze attached image and return a ready-to-insert analysis section.
    let image_section = if message_has_photo(&msg) {
        analyze_image(
            &bot,
            &msg,
            &format!("{text}\n\nHistory:\n\n{history}"),
            &groq,
        )
        .await
    } else {
        // No image attached -> empty image section
        String::new()
    };

    // Build base prompt including user text, image analysis, and history.
    let base_prompt = format!("{text}\n\n{image_section}History:\n\n{history}");

    // Define reasoning and main generation models.
    let reasoning_model = "openai/gpt-oss-20b";
    let main_model = "openai/gpt-oss-120b";

    // Provide full context to the reasoning model.
    let reasoning_user = format!("Full prompt+history:\n\n{base_prompt}");

    let reasoning_messages = vec![
        ChatMessage::new_text(Role::System, prompts.get(Prompt::Preprocess)),
        ChatMessage::new_text(Role::User, reasoning_user),
    ];

    // Run reasoning step with a limited retry strategy.
    let mut refined_result: Option<String> = None;

    for attempt in 0..2 {
        let resp = groq
            .chat(reasoning_model)
            .messages(reasoning_messages.clone())
            .max_completion_tokens(2000)
            .temperature(0.0)
            .send()
            .await;

        match resp {
            Ok(r) => {
                // Expect plain text output from reasoning model.
                if let MessageContent::Text(text) = &r.choices[0].message.content {
                    if attempt == 0 {
                        // First attempt failed parsing; retry once.
                    } else {
                        // Final fallback: accept raw text as refined prompt.
                        refined_result = Some(text.trim().to_string());
                        break;
                    }
                } else if attempt == 1 {
                    // Non-text content fallback.
                    refined_result = Some(base_prompt.clone());
                }
            }
            Err(e) => {
                // Abort reasoning on model error.
                error!("Reasoning model error (attempt {}): {e}", attempt);
                refined_result = None;
                break;
            }
        }
    }

    // Build final prompt for the main model.
    let prompt_for_main = format!(
        "Main lang is \"{user_lang}\":\n\nOriginal prompt: {}\n\nResource for you response: {}",
        text,
        refined_result.unwrap_or(history)
    );

    let groq_messages = vec![
        ChatMessage::new_text(Role::System, prompts.get(Prompt::ThinkAndFormat)),
        ChatMessage::new_text(Role::User, prompt_for_main.clone()),
    ];

    // Call main model to generate the final answer.
    let main_res_text = match groq
        .chat(main_model)
        .messages(groq_messages)
        .max_completion_tokens(3000)
        .temperature(0.0)
        .send()
        .await
    {
        Ok(response) => {
            if let MessageContent::Text(text) = &response.choices[0].message.content {
                escape_telegram_code_entities(text)
            } else {
                "Nothing".to_string()
            }
        }
        Err(e) => {
            // Abort on generation error.
            let _ = tx.rollback().await;
            keep.shutdown().await;

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

    // Stop typing indicator.
    keep.shutdown().await;

    // Send response to Telegram before committing DB changes.
    let send_result = {
        let req = if let Some(tid) = thread_id {
            bot.send_message(chat_id, main_res_text.clone())
                .message_thread_id(tid)
                .parse_mode(ParseMode::Html)
        } else {
            bot.send_message(chat_id, main_res_text.clone())
                .parse_mode(ParseMode::Html)
        };
        req.await
    };

    if let Err(e) = send_result {
        // Roll back if message delivery fails.
        error!("Telegram send failed: {e} — rolling back DB transaction.");
        let _ = tx.rollback().await;
        return Ok(());
    }

    // Persist user prompt and AI response.
    if let Err(e) = sqlx::query!(
        r#"
        INSERT INTO messages (user_telegram_id, chat_telegram_id, content, ia_response)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        msg_chat_id,
        text,
        main_res_text,
    )
    .execute(&mut *tx)
    .await
    {
        error!("Query failed when inserting message after send: {e}");
        let _ = tx.rollback().await;
        bot.send_message(chat_id, "Database error (couldn't save message).")
            .await?;
        return Ok(());
    }

    // Commit transaction.
    if let Err(e) = tx.commit().await {
        error!("Transaction commit failed: {e}");
        bot.send_message(chat_id, "Error saving data.").await?;
        return Ok(());
    }

    Ok(())
}
