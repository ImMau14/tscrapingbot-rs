// Handler for the search command

use crate::{
    handlers::{
        types::MessageRow,
        utils::{
            ChatActionKeepAlive, escape_telegram_code_entities, extract_user_info,
            fetch_simplified_body, format_messages_xml,
            llm::{run_main_model, run_reasoning_step},
            send_reply_or_plain,
        },
    },
    prompts::AiPrompt,
};
use groqai::GroqClient;
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatAction, ThreadId},
};
use tracing::error;

pub async fn search(
    bot: Bot,
    msg: Message,
    text: String,
    scrapedo_token: String,
    pool: PgPool,
    groq: GroqClient,
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
            "I can't reply to an empty message.",
            false,
            false,
        )
        .await?;
        return Ok(());
    }

    // Prompt helper to access predefined system prompts.
    let prompts = AiPrompt::new();

    // Validate and extract user information.
    let (user_id, user_lang, msg_chat_id) = match extract_user_info(&msg) {
        Ok(v) => v,
        Err(err_msg) => {
            // User-facing error, stop typing indicator, return
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, err_msg, false, false).await?;
            return Ok(());
        }
    };

    // Begin a new database transaction.
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("DB transaction error: {e}");
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Internal database error.", false, false).await?;
            return Ok(());
        }
    };

    // Retrieve recent messages for context.
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
            error!("Query failed: {e}.");
            let _ = tx.rollback().await;
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Database error", false, false).await?;
            return Ok(());
        }
    };

    let history = format_messages_xml(&messages, 0, false);

    // Ensure the provided text contains a valid URL.
    let url = match text.split_whitespace().next() {
        Some(url) => {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                error!("Search failed: Not URL to search");
                let _ = tx.rollback().await;
                keep.shutdown().await;
                send_reply_or_plain(
                    &bot,
                    &msg,
                    "Use a valid URL (http:// or https://).",
                    false,
                    false,
                )
                .await?;
                return Ok(());
            }

            let parsed_url = url.replace('&', "%26");

            &format!("http://api.scrape.do/?token={scrapedo_token}&url={parsed_url}")
        }
        None => {
            error!("Search failed: Not URL to search");
            let _ = tx.rollback().await;
            keep.shutdown().await;
            send_reply_or_plain(
                &bot,
                &msg,
                "Use a valid URL (http:// or https://).",
                false,
                false,
            )
            .await?;
            return Ok(());
        }
    };

    // Retrieve the simplified body of the web resource.
    let web_resource: String = match fetch_simplified_body(url).await {
        Ok(res) => res,
        Err(e) => {
            let err_text = e.clone();
            error!("Search failed: {}", err_text);
            let _ = tx.rollback().await;
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Search error.", false, false).await?;
            return Ok(());
        }
    };

    // Prepare the base prompt for the reasoning step.
    let base_prompt = format!("{text}\n\nWebResource:\n{web_resource}History:\n\n{history}");
    let reasoning_model = "openai/gpt-oss-20b";
    let main_model = "openai/gpt-oss-120b";

    // Run the reasoning model.
    let refined = match run_reasoning_step(&groq, &prompts, &base_prompt, reasoning_model).await {
        Some(v) => v,
        None => {
            // Fatal preprocessing error: rollback and notify
            let _ = tx.rollback().await;
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Error during preprocessing.", false, false).await?;
            return Ok(());
        }
    };

    // Construct the prompt for the main model.
    let prompt_for_main = format!(
        "Main lang is \"{user_lang}\":\n\nOriginal prompt: {}\n\nResource for you response: {}\n\nWebResource:\n{web_resource}",
        text, refined
    );

    // Execute the main language model.
    let raw_answer = match run_main_model(&groq, &prompts, &prompt_for_main, main_model).await {
        Ok(v) => v,
        Err(e) => {
            // Log error without dropping the exception.
            let err_text = e.to_string();
            // Model error: rollback and notify.
            let _ = tx.rollback().await;
            keep.shutdown().await;
            error!("Search failed: {}.", err_text);
            send_reply_or_plain(&bot, &msg, "Internal main model error.", false, false).await?;
            return Ok(());
        }
    };

    // Escape for Telegram HTML before sending and saving.
    let final_answer = escape_telegram_code_entities(&raw_answer);

    // Stop typing indicator BEFORE any DB awaits that could be after sending.
    keep.shutdown().await;

    // Build send request (no await yet)
    let send_req = send_reply_or_plain(&bot, &msg, final_answer.clone(), false, true);

    // Send message (await). Any error must be materialized & logged before later awaits.
    if let Err(e) = send_req.await {
        // Log error without dropping the exception.
        let err_text = e.to_string();
        error!(
            "Telegram send failed: {} — rolling back DB transaction.",
            err_text
        );
        let _ = tx.rollback().await;
        return Ok(());
    }

    // Save message into DB (await inside the branch)
    // Insert the sent message into the database.
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
        send_reply_or_plain(
            &bot,
            &msg,
            "Database error (couldn't save message).",
            false,
            false,
        )
        .await?;
        return Ok(());
    }

    // Commit; log and notify user on failure
    if let Err(e) = tx.commit().await {
        error!("Commit failed: {e}");
        send_reply_or_plain(&bot, &msg, "Error saving data.", false, false).await?;
    }

    Ok(())
}
