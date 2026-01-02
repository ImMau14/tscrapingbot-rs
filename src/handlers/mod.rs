mod ask;
use ask::ask;

mod reset;
use reset::reset;

pub mod types;
pub mod utils;

use crate::commands::Command;
use groqai::GroqClient;
use sqlx::postgres::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};
use tracing::info;

// NOTE: use `Bot` (not `AutoSend<Bot>`) so the code works without enabling
// teloxide's `auto-send` feature in Cargo.toml.
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    pool: PgPool,
    groq: GroqClient,
) -> ResponseResult<()> {
    info!("Update received: chat_id = {}", msg.chat.id);

    match cmd {
        Command::Ask(text) => ask(bot, msg, text, pool.clone(), groq.clone()).await?,
        Command::Repeat(text) => {
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::Reset => {
            reset(bot, msg, pool.clone()).await?;
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
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
