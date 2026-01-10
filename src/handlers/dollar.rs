// Fetches the current dollar price from the BCV website.

use crate::handlers::utils::{ChatActionKeepAlive, send_reply_or_plain};
use kuchiki::traits::*;
use reqwest;
use teloxide::{
    prelude::*,
    types::{ChatAction, ThreadId},
};
use tracing::error;

// Handles the /dollar command, retrieves price and sends reply.
pub async fn dollar(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let thread_id: Option<ThreadId> = msg.thread_id;

    // Start keep-alive typing indicator.
    let mut keep =
        ChatActionKeepAlive::spawn(bot.clone(), chat_id, thread_id, ChatAction::Typing, 4);

    // Fetch BCV homepage.
    let res = match reqwest::get("https://www.bcv.org.ve").await {
        Ok(val) => val,
        Err(e) => {
            // Handle request failure.
            keep.shutdown().await;
            error!("Could not retrieve the dollar page: {:?}", e);
            send_reply_or_plain(
                &bot,
                &msg,
                "Could not retrieve the dollar page.",
                false,
                false,
            )
            .await?;
            return Ok(());
        }
    };

    // Convert response body to text.
    let raw = match res.text().await {
        Ok(val) => val,
        Err(e) => {
            // Handle text conversion failure.
            keep.shutdown().await;
            error!("Could not convert the response to text: {e}");
            send_reply_or_plain(
                &bot,
                &msg,
                "Could not convert the response to text.",
                false,
                false,
            )
            .await?;
            return Ok(());
        }
    };

    // Parse HTML and extract dollar price.
    let dollar_price_opt: Option<f64> = {
        let document = kuchiki::parse_html().one(raw);

        // Select strong element inside #dolar.
        match document.select("#dolar strong") {
            Ok(mut nodes) => {
                if let Some(node_ref) = nodes.next() {
                    let text = node_ref.as_node().text_contents();
                    // Clean and parse price string.
                    match text.trim().replace(",", ".").parse::<f64>() {
                        Ok(val) => Some(val),
                        Err(e) => {
                            error!("Failed to convert the value to a number: {e}");
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Err(e) => {
                error!("Failed to run CSS selector on document: {:?}", e);
                None
            }
        }
    };

    // Send appropriate reply based on extraction result.
    match dollar_price_opt {
        Some(dollar_price) => {
            let message = format!("<b>BCV</b>: <code>{dollar_price} Bs.</code>");
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, message, false, true).await?;
            Ok(())
        }
        None => {
            keep.shutdown().await;
            send_reply_or_plain(&bot, &msg, "Failed to get dollar value.", false, false).await?;
            Ok(())
        }
    }
}
