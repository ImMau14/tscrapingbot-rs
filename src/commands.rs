use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "respond using AI")]
    Ask(String),

    #[command(description = "repeat text back to you")]
    Repeat(String),

    #[command(description = "reset the chat history.")]
    Reset,

    #[command(description = "the start command.")]
    Start,

    #[command(description = "get the dollar price at the BCV.")]
    Dollar,

    #[command(description = "respond using AI and your web resource")]
    Search(String),

    #[command(description = "display this text.")]
    Help,
}
