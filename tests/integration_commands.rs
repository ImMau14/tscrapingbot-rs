use groqai::GroqClient;
use serial_test::serial;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use teloxide::dptree;
use teloxide_tests::{MockBot, MockMessageText};
use tracing::error;
use tscrapingbot_rs::handlers::get_update_handler;

#[tokio::test]
#[serial]
async fn repeat_command_integration() {
    dotenvy::dotenv().ok();

    let groq_api_key =
        env::var("GROQ_API_KEY").expect("GROQ_API_KEY not set (check .env or environment)");
    let groq = match GroqClient::with_api_key(groq_api_key) {
        Ok(client) => client,
        Err(e) => {
            error!("The Groq client could not be started: {e}");
            return;
        }
    };

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL not set (check .env or environment)");
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("The database could not be connected");

    let mock = MockMessageText::new().text("/repeat hola");
    let handler = get_update_handler();

    let mut bot = MockBot::new(mock, handler);
    bot.dependencies(dptree::deps![groq, pool]);

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
        env::var("GROQ_API_KEY").expect("GROQ_API_KEY not set (check .env or environment)");
    let groq = match GroqClient::with_api_key(api_key) {
        Ok(client) => client,
        Err(e) => {
            error!("The Groq client could not be started: {e}");
            return;
        }
    };

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL not set (check .env or environment)");
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("The database could not be connected");

    let mock = MockMessageText::new().text("/help");
    let handler = get_update_handler();

    let mut bot = MockBot::new(mock, handler);
    bot.dependencies(dptree::deps![groq, pool]);

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
        env::var("GROQ_API_KEY").expect("GROQ_API_KEY not set (check .env or environment)");

    let groq1 = match GroqClient::with_api_key(api_key.clone()) {
        Ok(client) => client,
        Err(e) => {
            error!("The Groq client could not be started: {e}");
            return;
        }
    };

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL not set (check .env or environment)");
    let pool1: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("The database could not be connected");

    let mock = MockMessageText::new().text("/ask Write exactly: Hi");
    let handler = get_update_handler();

    let mut bot = MockBot::new(mock, handler);
    bot.dependencies(dptree::deps![groq1, pool1.clone()]);

    bot.dispatch().await;

    let binding = bot.get_responses();
    let last = binding.sent_messages.last().expect("no response");

    let text = last.text().unwrap_or_default();

    assert!(text.contains("Hi"), "Unexpected response: {text}");

    let groq2 = match GroqClient::with_api_key(api_key.clone()) {
        Ok(client) => client,
        Err(e) => {
            error!("The Groq client for reset could not be started: {e}");
            return;
        }
    };

    let pool2: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("The database could not be connected for reset");

    let mock_reset = MockMessageText::new().text("/reset");
    let handler_reset = get_update_handler();

    let mut reset_bot = MockBot::new(mock_reset, handler_reset);
    reset_bot.dependencies(dptree::deps![groq2, pool2]);

    reset_bot.dispatch().await;

    let reset_binding = reset_bot.get_responses();
    let reset_last = reset_binding
        .sent_messages
        .last()
        .expect("expected a response to /reset");

    let reset_text = reset_last.text().unwrap_or_default().to_lowercase();

    assert!(
        reset_text.contains("chat reset successfully")
            || reset_text.contains("the chat has already been reset")
            || reset_text.contains("already been reset"),
        "Unexpected reset response: {reset_text}"
    );
}
