// Command handling module with controlled concurrency.

mod ask;
use ask::ask;

mod reset;
use reset::reset;

mod search;
use search::search;

mod start;
use start::start;

mod dollar;
use dollar::dollar;

pub mod types;
pub mod utils;

use crate::{commands::Command, config::AppConfig};
use groqai::GroqClient;
use once_cell::sync::Lazy;
use sqlx::postgres::PgPool;
use std::{collections::HashMap, sync::Arc};
use teloxide::{dptree, filter_command, prelude::*, types::Message, utils::command::BotCommands};
use tokio::sync::{Mutex as TokioMutex, Semaphore};
use tracing::info;

// Executor controls command execution concurrency.
struct Executor {
    // Global concurrency limiter.
    semaphore: Arc<Semaphore>,

    // Per-user locks.
    // Each user key maps to a mutex that serializes their commands.
    user_locks: Arc<TokioMutex<HashMap<String, Arc<TokioMutex<()>>>>>,
}

impl Executor {
    // Create a new executor with a maximum number of concurrent commands.
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            user_locks: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    // Execute a task with concurrency guarantees.
    async fn run<F, Fut, R>(&self, user_key: String, f: F) -> R
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        // Acquire global concurrency slot.
        let permit = self.semaphore.acquire().await.expect("semaphore closed");

        // Obtain or create the per-user mutex.
        let lock_arc = {
            let mut map = self.user_locks.lock().await;
            map.entry(user_key.clone())
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };

        // Serialize execution for this user.
        let _user_guard = lock_arc.lock().await;

        // Execute task.
        let result = f().await;

        // Release global slot.
        drop(permit);

        // Cleanup user lock if no other references exist.
        if Arc::strong_count(&lock_arc) == 1 {
            let mut map = self.user_locks.lock().await;
            if let Some(existing) = map.get(&user_key)
                && Arc::ptr_eq(existing, &lock_arc)
            {
                map.remove(&user_key);
            }
        }

        result
    }
}

// Global executor instance.
static EXECUTOR: Lazy<Arc<Executor>> = Lazy::new(|| Arc::new(Executor::new(5)));

// Compute the serialization key for a message.
fn user_key_from_message(msg: &Message) -> String {
    if let Some(user) = msg.from.as_ref() {
        user.id.0.to_string()
    } else {
        msg.chat.id.to_string()
    }
}

// Main command handler.
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    pool: PgPool,
    groq: GroqClient,
    app_config: AppConfig,
) -> ResponseResult<()> {
    // Validate message author.
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => {
            bot.send_message(msg.chat.id, "The user could not be identified.")
                .await?;
            return Ok(());
        }
    };

    info!(
        "Update received: chat_id={}, from={:?}",
        msg.chat.id, user.id.0 as i64
    );

    let user_key = user_key_from_message(&msg);

    // Clone shared resources for the async task.
    let bot_clone = bot.clone();
    let msg_clone = msg.clone();
    let pool_clone = pool.clone();
    let groq_clone = groq.clone();

    EXECUTOR
        .run(user_key, move || {
            let bot = bot_clone.clone();
            let msg = msg_clone.clone();
            let pool = pool_clone.clone();
            let groq = groq_clone.clone();

            async move {
                match cmd {
                    Command::Ask(text) => {
                        if let Err(e) =
                            ask(bot, msg, text, pool, groq, app_config.models.clone()).await
                        {
                            tracing::error!("Ask command failed: {:?}", e);
                        }
                    }
                    Command::Repeat(text) => {
                        if let Err(e) = bot.send_message(msg.chat.id, text).await {
                            tracing::error!("Repeat command failed: {:?}", e);
                        }
                    }
                    Command::Reset => {
                        if let Err(e) = reset(bot, msg, pool).await {
                            tracing::error!("Reset command failed: {:?}", e);
                        }
                    }
                    Command::Start => {
                        if let Err(e) = start(bot, msg).await {
                            tracing::error!("Start command failed: {:?}", e);
                        }
                    }
                    Command::Dollar => {
                        if let Err(e) = dollar(bot, msg).await {
                            tracing::error!("Dollar command failed: {:?}", e);
                        }
                    }
                    Command::Search(text) => {
                        if let Err(e) = search(
                            bot,
                            msg,
                            text,
                            app_config.scrapedo_token.clone(),
                            pool,
                            groq,
                            app_config.models.clone(),
                        )
                        .await
                        {
                            tracing::error!("Search command failed: {:?}", e);
                        }
                    }
                    Command::Help => {
                        if let Err(e) = bot
                            .send_message(msg.chat.id, Command::descriptions().to_string())
                            .await
                        {
                            tracing::error!("Help command failed: {:?}", e);
                        }
                    }
                }
            }
        })
        .await;

    Ok(())
}

// Handler for private messages WITHOUT commands.
async fn handle_private_plain_text(
    bot: Bot,
    msg: Message,
    pool: PgPool,
    groq: GroqClient,
    app_config: AppConfig,
) -> ResponseResult<()> {
    // Prefer real text() (normal messages), fall back to caption() (media captions), else empty.
    let text = if let Some(t) = msg.text() {
        t.to_string()
    } else if let Some(c) = msg.caption() {
        c.to_string()
    } else {
        String::new()
    };

    // If user typed an explicit slash-command in a text message, ignore here (handled by commands branch).
    if msg.text().is_some() && text.starts_with('/') {
        return Ok(());
    }

    // Route to same command handler so concurrency/SQL logic remains unchanged.
    handle_command(bot, msg, Command::Ask(text), pool, groq, app_config).await
}

// Build the update handler tree.
pub fn get_update_handler() -> teloxide::dispatching::UpdateHandler<teloxide::RequestError> {
    teloxide::types::Update::filter_message().branch(
        dptree::entry()
            // Explicit bot commands.
            .branch(filter_command::<Command, _>().endpoint(handle_command))
            // Private chat messages: accept text OR caption OR photo -> Ask.
            .branch(
                dptree::filter(|msg: Message| {
                    msg.chat.is_private()
                        && (msg.text().is_some() || msg.caption().is_some() || msg.photo().is_some())
                        // if there's textual `text()` and it starts with '/', treat as command and ignore here
                        && !msg.text().map(|t| t.starts_with('/')).unwrap_or(false)
                })
                .endpoint(handle_private_plain_text),
            ),
    )
}
