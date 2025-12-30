use serial_test::serial;
use teloxide_tests::{MockBot, MockMessageText};
use tscrapingbot_rs::gemini::Gemini;
use tscrapingbot_rs::handlers::get_update_handler;

use std::env;
use std::sync::Arc;
use teloxide::dptree;

#[tokio::test]
#[serial]
async fn repeat_command_integration() {
    dotenvy::dotenv().ok();

    let api_key =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set (check .env or environment)");
    let gemini = Arc::new(Gemini::new(api_key));

    let mock = MockMessageText::new().text("/repeat hola");
    let handler = get_update_handler();

    let mut bot = MockBot::new(mock, handler);
    bot.dependencies(dptree::deps![gemini]);

    bot.dispatch().await;

    let binding = bot.get_responses();
    let last = binding
        .sent_messages
        .last()
        .expect("At least 1 sent message was expected");

    assert_eq!(last.text(), Some("hola"));
}

#[tokio::test]
#[serial]
async fn help_command_integration() {
    dotenvy::dotenv().ok();

    let api_key =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set (check .env or environment)");
    let gemini = Arc::new(Gemini::new(api_key));

    let mock = MockMessageText::new().text("/help");
    let handler = get_update_handler();

    let mut bot = MockBot::new(mock, handler);
    bot.dependencies(dptree::deps![gemini]);

    bot.dispatch().await;

    let binding = bot.get_responses();
    let last = binding.sent_messages.last().expect("no response");

    let text = last.text().unwrap_or_default();

    assert!(
        text.contains("Available commands") || text.to_lowercase().contains("available"),
        "Unexpected help text: {text}"
    );
}

#[tokio::test]
#[serial]
async fn ask_command_integration() {
    dotenvy::dotenv().ok();

    let api_key =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set (check .env or environment)");
    let gemini = Arc::new(Gemini::new(api_key));

    let mock = MockMessageText::new().text("/ask Write exactly: Hi");
    let handler = get_update_handler();

    let mut bot = MockBot::new(mock, handler);
    bot.dependencies(dptree::deps![gemini]);

    bot.dispatch().await;

    let binding = bot.get_responses();
    let last = binding.sent_messages.last().expect("no response");

    let text = last.text().unwrap_or_default();

    assert!(text.contains("Hi"), "Unexpected response: {text}");
}
