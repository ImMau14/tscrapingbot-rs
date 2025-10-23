mod ask;
use ask::ask;

use crate::commands::Command;
use crate::gemini::Gemini;
use std::sync::Arc;
use teloxide::utils::command::BotCommands;
use teloxide::{prelude::*, types::ChatAction};
use tracing::info;

// NOTE: use `Bot` (not `AutoSend<Bot>`) so the code works without enabling
// teloxide's `auto-send` feature in Cargo.toml.
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    gemini: Arc<Gemini>,
) -> ResponseResult<()> {
    info!("Update received: chat_id = {}", msg.chat.id);
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    match cmd {
        Command::Ask(text) => ask(bot, msg, text, gemini.clone()).await,
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Repeat(text) => {
            bot.send_message(msg.chat.id, text).await?;
        }
    }
    Ok(())
}

pub fn get_update_handler() -> teloxide::dispatching::UpdateHandler<teloxide::RequestError> {
    teloxide::types::Update::filter_message().branch(
        teloxide::dptree::entry()
            .filter_command::<Command>()
            .endpoint(handle_command),
    )
}
