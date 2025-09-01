pub mod commands;
pub mod config;
pub mod handlers;
pub mod server;
pub mod trace;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

use config::AppConfig;
use handlers::get_update_handler;
use std::net::SocketAddr;
use teloxide::dispatching::Dispatcher;
use teloxide::error_handlers::LoggingErrorHandler;
use teloxide::prelude::*;
use teloxide::update_listeners::webhooks;
use tokio::signal;
use trace::init_tracing;
use tracing::{error, info};

pub async fn run() -> Result<(), BoxError> {
    init_tracing();

    let cfg = match AppConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("Configuration error: {}", e);
            return Err(Box::new(e) as BoxError);
        }
    };

    info!("Starting bot (hosting = {})", cfg.hosting);

    let bot = Bot::new(cfg.token.clone());

    let handler = get_update_handler();
    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .enable_ctrlc_handler()
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
            return Err(Box::new(config::ConfigError::MissingEnv("WEBHOOK_URL")) as BoxError);
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    info!("Configuring webhook for URL: {}", webhook_url);

    let options = webhooks::Options::new(addr, webhook_url.clone());
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

    let app = server::build_router(Some(webhook_router));

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

    let server_handle = tokio::spawn(async move {
        if let Err(e) = server_with_shutdown.await {
            error!("Axum server error: {}", e);
        }
    });

    dispatcher
        .dispatch_with_listener(update_listener, LoggingErrorHandler::new())
        .await;

    if let Err(e) = server_handle.await {
        error!("Server task join error: {}", e);
    }

    info!("Bot shutdown complete.");
    Ok(())
}
