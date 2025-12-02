// Manager that keeps a chat action being sent periodically.

use teloxide::{
    prelude::*,
    types::{ChatAction, ChatId, ThreadId},
};
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

impl ChatActionKeepAlive {
    // Spawn background task that periodically sends the given ChatAction.
    // thread_id is optional; when Some(tid) the chat action will be sent
    // With .message_thread_id(tid) so it appears in the forum topic.
    pub fn spawn(
        bot: Bot,
        chat_id: ChatId,
        thread_id: Option<ThreadId>,
        action: ChatAction,
        interval_secs: u64,
    ) -> Self {
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Build the request, attach message_thread_id only if present.
                        let send_req = if let Some(tid) = thread_id {
                            bot.send_chat_action(chat_id, action).message_thread_id(tid)
                        } else {
                            bot.send_chat_action(chat_id, action)
                        };

                        if let Err(err) = send_req.await {
                            tracing::warn!("send_chat_action failed: {:?}", err);
                        }
                    }

                    // Stop signal received.
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });

        Self {
            stop_tx: Some(stop_tx),
            handle: Some(handle),
        }
    }

    // Gracefully stop the background task and await its completion.
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }

        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
    }
}

impl Drop for ChatActionKeepAlive {
    fn drop(&mut self) {
        // Request stop synchronously if still available.
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }

        // If the task still exists, abort it to avoid leaking.
        if let Some(h) = &self.handle {
            h.abort();
        }
    }
}
