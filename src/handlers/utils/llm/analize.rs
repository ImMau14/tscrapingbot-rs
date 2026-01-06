// Keep these small and testable: other handlers can call them directly.

use crate::prompts::{AiPrompt, Prompt};
use groqai::{ChatMessage, GroqClient, MessageContent, Role};
use tracing::error;

// Run the "reasoning" / preprocessing model with a minimal retry strategy.
pub async fn run_reasoning_step(
    groq: &GroqClient,
    prompts: &AiPrompt,
    base_prompt: &str,
    reasoning_model: &str,
) -> Option<String> {
    // Build a simple conversation for the reasoning model
    let reasoning_user = format!("Full prompt+history:\n\n{base_prompt}");
    let messages = vec![
        ChatMessage::new_text(Role::System, prompts.get(Prompt::Preprocess)),
        ChatMessage::new_text(Role::User, reasoning_user),
    ];

    // Up to 2 attempts: useful for transient model hiccups or non-text outputs
    for attempt in 0..2 {
        match groq
            .chat(reasoning_model)
            .messages(messages.clone())
            .max_completion_tokens(2000)
            .temperature(0.0)
            .send()
            .await
        {
            Ok(resp) => {
                // Prefer textual outputs; trim whitespace
                if let MessageContent::Text(text) = &resp.choices[0].message.content {
                    return Some(text.trim().to_string());
                } else if attempt == 1 {
                    // Final fallback: return the original base prompt so main model still runs
                    return Some(base_prompt.to_string());
                }
                // Otherwise retry once more
            }
            Err(e) => {
                // Log and abort: caller needs to handle rollback/notify user
                error!("Reasoning model error (attempt {}): {e}", attempt);
                return None;
            }
        }
    }

    // Unreachable under normal flow, keep as explicit fallback
    None
}

// Call the main generation model and return raw text (no Telegram escaping).
pub async fn run_main_model(
    groq: &GroqClient,
    prompts: &AiPrompt,
    prompt_for_main: &str,
    main_model: &str,
) -> Result<String, String> {
    let messages = vec![
        ChatMessage::new_text(Role::System, prompts.get(Prompt::ThinkAndFormat)),
        ChatMessage::new_text(Role::User, prompt_for_main.to_string()),
    ];

    let resp = groq
        .chat(main_model)
        .messages(messages)
        .max_completion_tokens(3000)
        .temperature(0.0)
        .send()
        .await
        .map_err(|e| format!("Main model error: {e}"))?;

    // Return trimmed text (handler will perform Telegram escaping before sending)
    if let MessageContent::Text(text) = &resp.choices[0].message.content {
        Ok(text.trim().to_string())
    } else {
        Ok(String::new())
    }
}
