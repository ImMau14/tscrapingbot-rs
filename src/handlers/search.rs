// Handler for the search command

use crate::{
    config::Models,
    handlers::{
        types::MessageRow,
        utils::{
            ChatActionKeepAlive, escape_telegram_code_entities, extract_user_info,
            fetch_simplified_body, send_chat_with_history, send_reply_or_plain, truncate_text,
        },
    },
    prompts::{AiPrompt, Prompt},
};
use groqai::{ChatMessage, GroqClient, MessageContent, Role};
use regex::Regex;
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatAction, ThreadId},
};
use tracing::{error, info};

// Shortened copy of the scraped page kept in history (~1K tokens, sized for
// Groq free-tier TPM). The full simplified body still reaches the model in
// the current request.
const STORED_WEB_RESOURCE_CHARS: usize = 4_000;

pub async fn search(
    bot: Bot,
    msg: Message,
    text: String,
    scrapedo_token: String,
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
            "I can't reply to an empty message. Use /search <url> <query>.",
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

    // Retrieve recent messages for context.
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
            error!("Query failed: {e}.");
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Database error", false, false).await?;
            return Ok(());
        }
    };

    // Ensure the provided text contains a valid URL.
    let url_str = match text.split_whitespace().next() {
        Some(candidate) => {
            if !(candidate.starts_with("http://") || candidate.starts_with("https://")) {
                error!("Search failed: Not URL to search");
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

            // Encode ampersands to keep query safe.
            candidate.replace('&', "%26")
        }
        None => {
            error!("Search failed: Not URL to search");
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
    info!("Fetching simplified body");
    let web_resource: String = match fetch_simplified_body(&format!(
        "http://api.scrape.do/?token={scrapedo_token}&url={url_str}"
    ))
    .await
    {
        Ok(res) => {
            let re = Regex::new(r"\{[^{}]*\}").unwrap();

            if re.find(&res).is_some() && res.contains(r#""StatusCode":400"#) {
                match fetch_simplified_body(&url_str).await {
                    Ok(res) => res,
                    Err(e) => {
                        let err_text = e.clone();
                        error!("Search failed: {}", err_text);
                        keep.shutdown().await;
                        send_reply_or_plain(&bot, &msg, "Search error.", false, false).await?;
                        return Ok(());
                    }
                }
            } else {
                res
            }
        }
        Err(e) => {
            let err_text = e.clone();
            error!("Search failed: {}", err_text);
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Search error.", false, false).await?;
            return Ok(());
        }
    };

    // Build a single conversation array and use only the main model.
    let main_model = &models.thinking;
    let sec_model = &models.preprocessing;
    let system_prompt = prompts.get(Prompt::ThinkAndFormat);

    // Historical turns stay chronological (oldest-first); the helper trims them
    // to a token budget and retries with a smaller one on context overflow.
    messages.reverse();

    // Add the user prompt (HTML will be passed separately below).
    let current_user_msg = format!("Main lang is \"{user_lang}\":\n\nUser prompt: {}", text);

    // Pass the fetched HTML/body as a separate user message to improve tokenization/context handling.
    let web_msg = format!("WebResource:\n{web_resource}");

    let raw_answer = match send_chat_with_history(
        &groq,
        main_model,
        system_prompt,
        &messages,
        vec![
            ChatMessage::new_text(Role::User, current_user_msg),
            ChatMessage::new_text(Role::User, web_msg),
        ],
        2000,
    )
    .await
    {
        Ok(answer) => answer,
        Err(e) => {
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, format!("Error: {e}."), false, false).await?;
            return Ok(());
        }
    };

    let final_answer = escape_telegram_code_entities(&raw_answer);

    keep.shutdown().await;

    let send_req = send_reply_or_plain(&bot, &msg, final_answer.clone(), false, true);

    if let Err(e) = send_req.await {
        let err_text = e.to_string();
        if err_text.to_lowercase().contains("parse") || err_text.to_lowercase().contains("parsing")
        {
            error!("Telegram parse error: {}.", err_text);

            // Ask preprocessing model to try to apply HTML/formatting to the raw model output
            let fmt_res = match groq
                .chat(sec_model)
                .messages(vec![
                    ChatMessage::new_text(Role::System, prompts.get(Prompt::Html)),
                    ChatMessage::new_text(Role::User, raw_answer.clone()),
                ])
                .max_completion_tokens(3000)
                .temperature(0.0)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    keep.shutdown().await;
                    send_reply_or_plain(&bot, &msg, format!("Error: {e}."), false, false).await?;
                    return Ok(());
                }
            };

            let fmt_text = if let MessageContent::Text(text) = &fmt_res.choices[0].message.content {
                text.trim().to_string()
            } else {
                String::new()
            };

            let reformated_answer = escape_telegram_code_entities(&fmt_text);

            let fmt_req = send_reply_or_plain(&bot, &msg, &reformated_answer, false, true);

            if let Err(e) = fmt_req.await {
                error!("Telegram send failed: {e} — no DB transaction to roll back.");
                return Ok(());
            }

            return Ok(());
        }

        error!(
            "Telegram send failed: {} — no DB transaction to rollback.",
            err_text
        );
        return Ok(());
    }

    if let Err(e) = sqlx::query!(
        r#"
        INSERT INTO messages (user_telegram_id, chat_telegram_id, content, ia_response)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        msg_chat_id,
        // Store a shortened version of the scraped page: the full body would
        // replay into every future request and eat the context window.
        format!(
            "{text}\n\nWeb Resource:\n\n{}",
            truncate_text(&web_resource, STORED_WEB_RESOURCE_CHARS)
        ),
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
        return Ok(());
    }

    Ok(())
}
