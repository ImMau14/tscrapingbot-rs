// Context budget helpers: keep conversation history inside the model's token
// window and stop scraped pages from drowning the real conversation.

use crate::handlers::types::MessageRow;
use groqai::{ChatMessage, GroqClient, MessageContent, Role};
use tracing::error;

/// Approximate tokens for a piece of text (chars / 4 is a safe heuristic for
/// mixed-language chat text; HTML-heavy scrapes tokenise slightly worse).
pub fn estimate_tokens(s: &str) -> usize {
    s.chars().count() / 4
}

/// Truncation marker kept short on purpose: it is stored inside `messages.content`
/// and replayed into every future request.
const TRUNCATION_MARKER: &str = "\n… [content truncated] …\n";

/// Keep the head and the tail of `s`, dropping the middle once it exceeds
/// `max_chars`. Head-first content is what the model needs most; the tail
/// preserves recent entries from listings (forums, proxy lists, logs).
pub fn truncate_text(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars || max_chars == 0 {
        return s.to_string();
    }

    let marker_len = TRUNCATION_MARKER.chars().count();
    if max_chars <= marker_len + 4 {
        // Too small to keep a tail: keep the head only.
        return s.chars().take(max_chars).collect();
    }

    let head = max_chars * 3 / 4;
    let tail = max_chars - head - marker_len;

    let mut out = String::with_capacity(max_chars);
    out.extend(s.chars().take(head));
    out.push_str(TRUNCATION_MARKER);
    out.extend(s.chars().skip(total - tail));
    out
}

/// Select the newest history rows that fit inside `max_chars` of combined
/// user + assistant text. `rows` must be newest-first (the order returned by
/// `get_recent_messages`). The newest row is always kept, even when oversized.
pub fn trim_history_to_budget(rows: &[MessageRow], max_chars: usize) -> Vec<MessageRow> {
    let mut used: usize = 0;
    let mut kept: Vec<MessageRow> = Vec::new();

    for row in rows {
        let size = row.content.as_deref().map_or(0, |c| c.chars().count())
            + row.ia_response.as_deref().map_or(0, |r| r.chars().count());

        if kept.is_empty() || used + size <= max_chars {
            kept.push(row.clone());
            used += size;
        } else {
            break;
        }
    }

    kept
}

/// Groq reports context overflow as an API error with one of these phrases.
/// Narrow on purpose: "limit"/"token" alone would also match rate-limit text.
pub fn is_context_length_error(e: &str) -> bool {
    let e = e.to_lowercase();
    const NEEDLES: &[&str] = &[
        "reduce the length",
        "maximum context",
        "context length",
        "context_length",
    ];
    NEEDLES.iter().any(|n| e.contains(n))
}

/// Groq free tier caps tokens per minute (TPM) and rejects oversize requests
/// with 413 "Payload Too Large". Match those so we can retry smaller too.
pub fn is_tpm_error(e: &str) -> bool {
    let e = e.to_lowercase();
    const NEEDLES: &[&str] = &[
        "tokens per minute",
        "(tpm)",
        "tpm: limit",
        "payload too large",
        "reduce your message size",
    ];
    NEEDLES.iter().any(|n| e.contains(n))
}

/// History char budgets tried in order when the model rejects the request as
/// too long. Sized for Groq free tier (8K TPM): the first budget keeps the
/// request near 4-5K tokens so input + 2K completion stays under the limit;
/// each fallback drops older rows until only the current exchange remains.
const HISTORY_BUDGETS_CHARS: &[usize] = &[16_000, 8_000, 4_000];

/// Send a chat completion with a bounded history.
///
/// `history` must be chronological (oldest-first, as callers keep it after
/// reversing the DB result). `current_msgs` holds the current user turn and
/// any extra content (web resource, image analysis). On a context-length
/// error the request is retried with a smaller history budget; on any other
/// error the message is returned as-is.
pub async fn send_chat_with_history(
    groq: &GroqClient,
    model: &str,
    system_prompt: String,
    history: &[MessageRow],
    current_msgs: Vec<ChatMessage>,
    max_completion_tokens: u32,
) -> Result<String, String> {
    let mut last_err: Option<String> = None;

    // Trim newest-first (keep the freshest context), then rebuild chronological.
    let mut newest_first = history.to_vec();
    newest_first.reverse();

    for budget in HISTORY_BUDGETS_CHARS {
        let mut kept = trim_history_to_budget(&newest_first, *budget);
        kept.reverse();

        let mut convo: Vec<ChatMessage> =
            Vec::with_capacity(kept.len() * 2 + current_msgs.len() + 1);
        convo.push(ChatMessage::new_text(Role::System, system_prompt.clone()));

        for row in &kept {
            if let Some(ref user_content) = row.content {
                convo.push(ChatMessage::new_text(Role::User, user_content.clone()));
            }
            if let Some(ref assistant_content) = row.ia_response {
                convo.push(ChatMessage::new_text(
                    Role::Assistant,
                    assistant_content.clone(),
                ));
            }
        }
        convo.extend(current_msgs.clone());

        match groq
            .chat(model)
            .messages(convo)
            .max_completion_tokens(max_completion_tokens)
            .temperature(0.0)
            .send()
            .await
        {
            Ok(resp) => {
                return Ok(
                    if let MessageContent::Text(text) = &resp.choices[0].message.content {
                        text.trim().to_string()
                    } else {
                        String::new()
                    },
                );
            }
            Err(e) => {
                let msg = e.to_string();
                if !is_context_length_error(&msg) && !is_tpm_error(&msg) {
                    return Err(msg);
                }
                last_err = Some(msg);
                error!(
                    "Context/TPM error with {} chars budget; retrying smaller",
                    budget
                );
            }
        }
    }

    Err(last_err
        .map(|e| {
            if is_tpm_error(&e) {
                format!(
                    "Groq rate limit (free tier: 8000 tokens/minute). Wait a minute and try again, or /reset to shorten the conversation. {e}"
                )
            } else {
                e
            }
        })
        .unwrap_or_else(|| "Model request failed".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(user: &str, assistant: &str) -> MessageRow {
        MessageRow {
            content: Some(user.to_string()),
            ia_response: Some(assistant.to_string()),
        }
    }

    #[test]
    fn truncate_keeps_head_and_tail() {
        let s = "a".repeat(1000);
        let out = truncate_text(&s, 200);
        assert!(out.chars().count() <= 200);
        assert!(out.starts_with("aaa"));
        assert!(out.ends_with("aaa"));
        assert!(out.contains("truncated"));
    }

    #[test]
    fn truncate_short_text_untouched() {
        let s = "short text";
        assert_eq!(truncate_text(s, 100), s);
    }

    #[test]
    fn truncate_tiny_budget_keeps_head() {
        let s = "a".repeat(100);
        let out = truncate_text(&s, 8);
        assert!(out.chars().count() <= 8);
        assert!(!out.contains("truncated"));
    }

    #[test]
    fn trim_keeps_newest_within_budget() {
        let rows = vec![
            row("user newest", "resp newest"),
            row("old one", "resp old"),
        ];
        let kept = trim_history_to_budget(&rows, 10);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].content.as_deref(), Some("user newest"));
    }

    #[test]
    fn trim_always_keeps_newest_even_oversized() {
        let rows = vec![
            row(
                "huge content that exceeds any budget we could set here",
                "x",
            ),
            row("small", "small"),
        ];
        let kept = trim_history_to_budget(&rows, 5);
        assert_eq!(kept.len(), 1);
        assert!(
            kept[0]
                .content
                .as_deref()
                .unwrap_or_default()
                .starts_with("huge")
        );
    }

    #[test]
    fn context_error_detection() {
        assert!(is_context_length_error(
            "Please reduce the length of the messages or completion parameters, as the total tokens exceed the maximum context length"
        ));
        assert!(!is_context_length_error("Rate limited - too many requests"));
        assert!(!is_context_length_error("Invalid API key: bad"));
    }

    #[test]
    fn tpm_error_detection() {
        assert!(is_tpm_error(
            "413 Payload Too Large: Request too large for model on tokens per minute (TPM): Limit 8000, Requested 8963"
        ));
        assert!(is_tpm_error(
            "please reduce your message size and try again"
        ));
        assert!(!is_tpm_error("Rate limited - too many requests"));
    }
}
