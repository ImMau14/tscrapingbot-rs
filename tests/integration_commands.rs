use serial_test::serial;
use teloxide_tests::{MockBot, MockMessageText};

use tscrapingbot_rs::handlers::get_update_handler;

#[tokio::test]
#[serial]
async fn repeat_command_integration() {
    let mock = MockMessageText::new().text("/repeat hola");
    let handler = get_update_handler();
    let mut bot = MockBot::new(mock, handler);
    bot.dispatch().await;

    let responses = bot.get_responses();
    let last = responses
        .sent_messages
        .last()
        .expect("At least 1 sent message was expected");

    assert_eq!(last.text(), Some("hola"));
}

#[tokio::test]
#[serial]
async fn help_command_integration() {
    let mock = MockMessageText::new().text("/help");
    let handler = get_update_handler();
    let mut bot = MockBot::new(mock, handler);
    bot.dispatch().await;

    let responses = bot.get_responses();
    let last = responses.sent_messages.last().expect("no response");
    let text = last.text().unwrap_or_default();

    assert!(
        text.contains("Available commands") || text.to_lowercase().contains("available"),
        "Unexpected help text: {text}"
    );
}
