use std::{env, net::SocketAddr};

use axum::{Router, routing::get};
use dotenvy::dotenv;
use serde_json::json;
use teloxide::{
    dispatching::{Dispatcher, UpdateHandler},
    dptree,
    error_handlers::LoggingErrorHandler,
    prelude::*,
    types::Update,
    update_listeners::webhooks,
    utils::command::BotCommands,
};
use thiserror::Error;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Error, Debug)]
enum ConfigError {
    #[error("missing environment variable: {0}")]
    MissingEnv(&'static str),
    #[error("invalid HOSTING value (expected true|false): {0}")]
    InvalidHosting(String),
    #[error("invalid WEBHOOK_URL: {0}")]
    InvalidWebhookUrl(String),
}

#[derive(Clone)]
struct AppConfig {
    token: String,
    hosting: bool,
    webhook_url: Option<url::Url>,
    port: u16,
}

impl AppConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let _ = dotenv();

        let token =
            env::var("TELOXIDE_TOKEN").map_err(|_| ConfigError::MissingEnv("TELOXIDE_TOKEN"))?;

        let hosting_raw = env::var("HOSTING").map_err(|_| ConfigError::MissingEnv("HOSTING"))?;
        let hosting = match hosting_raw.to_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            other => return Err(ConfigError::InvalidHosting(other.to_string())),
        };

        let webhook_url = match env::var("WEBHOOK_URL") {
            Ok(s) if !s.trim().is_empty() => {
                let parsed =
                    url::Url::parse(&s).map_err(|_| ConfigError::InvalidWebhookUrl(s.clone()))?;
                Some(parsed)
            }
            _ => None,
        };

        let port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080u16);

        Ok(AppConfig {
            token,
            hosting,
            webhook_url,
            port,
        })
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "repeat text back to you")]
    Repeat(String),

    #[command(description = "display this text.")]
    Help,
}

// NOTE: use `Bot` (not `AutoSend<Bot>`) so the code works without enabling
// teloxide's `auto-send` feature in Cargo.toml.
async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
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

fn get_update_handler() -> UpdateHandler<teloxide::RequestError> {
    Update::filter_message().branch(
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(handle_command),
    )
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(json!({ "status": "ok" }))
}

fn init_tracing() {
    // Use RUST_LOG, fallback to info if not set
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), BoxError> {
    init_tracing();

    let cfg = match AppConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("Configuration error: {}", e);
            return Err(Box::new(e) as BoxError);
        }
    };

    info!("Starting bot (hosting = {})", cfg.hosting);

    // create bot (cloneable)
    let bot = Bot::new(cfg.token.clone());

    let handler = get_update_handler();
    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .enable_ctrlc_handler() // teloxide default ctrlc handler
        .build();

    if !cfg.hosting {
        info!("Running in polling mode (local development).");
        info!("Bot started");
        dispatcher.dispatch().await;
        info!("Dispatcher exited (polling mode).");
        return Ok(());
    }

    // HOSTING == true path
    let webhook_url = match cfg.webhook_url.clone() {
        Some(url) => url,
        None => {
            error!("HOSTING=true but WEBHOOK_URL not provided");
            return Err(Box::new(ConfigError::MissingEnv("WEBHOOK_URL")) as BoxError);
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    info!("Configuring webhook for URL: {}", webhook_url);
    // Build options and get (update_listener, stop_future, router)
    let options = webhooks::Options::new(addr, webhook_url.clone());

    // rename the first returned listener to `update_listener` to avoid shadowing/confusion
    let (update_listener, stop_future, webhook_router) =
        match webhooks::axum_to_router(bot.clone(), options).await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to configure webhook: {}", e);
                return Err(Box::new(e) as BoxError);
            }
        };

    info!("Webhook configured");
    info!("Bot started");

    let app = Router::new()
        .route("/health", get(health_handler))
        .merge(webhook_router);

    // bind TCP listener for axum server (note: teloxide gave us `update_listener` above)
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    let shutdown_signal = async {
        let ctrl = signal::ctrl_c();
        #[cfg(unix)]
        {
            let mut term_stream =
                match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                    Ok(s) => s,
                    Err(err) => {
                        error!("Failed to register SIGTERM handler: {}", err);
                        ctrl.await.expect("ctrl_c failed");
                        return;
                    }
                };

            tokio::select! {
                _ = ctrl => {},
                _ = term_stream.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            ctrl.await.expect("ctrl_c failed");
        }
    };

    let server_with_shutdown = server.with_graceful_shutdown(async {
        tokio::select! {
            _ = shutdown_signal => {
                info!("Shutdown signal received (SIGINT/SIGTERM). Stopping listener & server.");
            }
            _ = stop_future => {
                info!("Listener stop_future resolved.");
            }
        }
    });

    // Spawn server
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server_with_shutdown.await {
            error!("Axum server error: {}", e);
        }
    });

    // Run dispatcher with the teloxide update listener (not the TcpListener)
    dispatcher
        .dispatch_with_listener(update_listener, LoggingErrorHandler::new())
        .await;

    // wait server task to finish
    if let Err(e) = server_handle.await {
        error!("Server task join error: {}", e);
    }

    info!("Bot shutdown complete.");
    Ok(())
}
