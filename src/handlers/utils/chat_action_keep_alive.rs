// Manager that keeps a chat action being sent periodically.

use teloxide::{prelude::*, types::ChatAction};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{Duration, interval},
};

pub struct ChatActionKeepAlive {
    // Sender to signal the background task to stop.
    stop_tx: Option<oneshot::Sender<()>>,

    // Handle to the spawned Tokio task.
    handle: Option<JoinHandle<()>>,
}

// The manager
impl ChatActionKeepAlive {
    // Spawn background task that periodically sends the given ChatAction.
    pub fn spawn(bot: Bot, chat_id: ChatId, action: ChatAction, interval_secs: u64) -> Self {
        // Create a oneshot channel to request shutdown.
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

        // Spawn an asynchronous task that sends the chat action on a ticker.
        let handle = tokio::spawn(async move {
            // Create a periodic ticker using the provided interval.
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                tokio::select! {
                    // On each tick, attempt to send the chat action.
                    _ = ticker.tick() => {
                        if let Err(err) = bot.send_chat_action(chat_id, action).await {
                            // Log a warning if sending fails.
                            tracing::warn!("send_chat_action failed: {:?}", err);
                        }
                    }

                    // Break the loop when a stop signal is received.
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });

        // Return the manager containing the stop sender and task handle.
        Self {
            stop_tx: Some(stop_tx),
            handle: Some(handle),
        }
    }

    // Gracefully stop the background task and await its completion.
    pub async fn shutdown(&mut self) {
        // Send stop signal if available; ignore send error.
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }

        // Await the task if we have its handle; ignore join error.
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
    }
}

// Fallback cleanup to ensure the task is stopped on Drop.
impl Drop for ChatActionKeepAlive {
    fn drop(&mut self) {
        // Try to notify the task to stop.
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }

        // Abort the task if it still exists.
        if let Some(h) = &self.handle {
            h.abort();
        }
    }
}
