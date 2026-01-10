// Handler for the /reset command.

use crate::handlers::utils::{ChatActionKeepAlive, send_reply_or_plain};
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatAction, ThreadId},
};
use tracing::error;

pub async fn reset(bot: Bot, msg: Message, pool: PgPool) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    let user = match msg.from {
        Some(ref u) => u,
        None => {
            send_reply_or_plain(
                &bot,
                &msg,
                "The user could not be identified.",
                false,
                false,
            )
            .await?;
            return Ok(());
        }
    };

    let user_id: i64 = user.id.0 as i64;
    let msg_chat_id: i64 = thread_id.map(|tid| tid.0.0 as i64).unwrap_or(chat_id.0);

    match sqlx::query!(
        r#"
        UPDATE messages
        SET is_cleared = TRUE
        WHERE user_telegram_id = $1
          AND chat_telegram_id = $2
          AND deleted_at IS NULL
          AND is_cleared = FALSE
        "#,
        user_id,
        msg_chat_id
    )
    .execute(&pool)
    .await
    {
        Ok(res) => {
            let affected = res.rows_affected();
            keep.shutdown().await;

            if affected > 0 {
                let text = "Chat reset successfully.";
                send_reply_or_plain(&bot, &msg, text, false, false).await?;
            } else {
                let text = "The chat has already been reset.";
                send_reply_or_plain(&bot, &msg, text, false, false).await?;
            }
            Ok(())
        }
        Err(e) => {
            error!("Failed to reset messages: {e}");
            let err_text = "Internal database error while clearing messages.";
            keep.shutdown().await;

            send_reply_or_plain(&bot, &msg, err_text, false, false).await?;
            Ok(())
        }
    }
}
