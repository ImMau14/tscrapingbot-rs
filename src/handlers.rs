use crate::commands::Command;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::info;

// NOTE: use `Bot` (not `AutoSend<Bot>`) so the code works without enabling
// teloxide's `auto-send` feature in Cargo.toml.
pub async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    info!("Update received: chat_id = {}", msg.chat.id);
    match cmd {
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
