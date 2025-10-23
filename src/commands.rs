use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "respond using AI")]
    Ask(String),

    #[command(description = "repeat text back to you")]
    Repeat(String),

    #[command(description = "display this text.")]
    Help,
}
