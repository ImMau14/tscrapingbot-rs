// Image analysis helper that downloads a Telegram photo and sends it to a vision LLM.

use crate::handlers::types::MessageRow;
use base64::{Engine as _, engine::general_purpose};
use groqai::{ChatMessage, GroqClient, ImageUrl, MessageContent, MessagePart, Role};
use reqwest::Client;
use serde_json::{Value, json};
use teloxide::{
    prelude::*,
    types::{FileId, Message},
};
use tracing::error;

// Analyzes a Telegram image using a vision model, guided by the user prompt.
pub async fn analyze_image(
    bot: &Bot,
    msg: &Message,
    user_prompt: &str,
    system_prompt: &str,
    history: Vec<MessageRow>,
    groq: &GroqClient,
    vision_model: &str,
) -> String {
    // Vision-capable model identifier.
    let mut image_section = String::new();

    // Extract the largest available photo from the message.
    if let Some(file_id) = largest_photo_file_id(msg) {
        // Resolve the Telegram file path.
        if let Some(file_path) = get_telegram_file_path(bot, file_id).await {
            // Download image bytes into memory.
            match download_telegram_file_bytes(bot, &file_path).await {
                Ok(img_bytes) => {
                    // Detect image MIME type.
                    let mime = detect_image_mime(&img_bytes);
                    // Encode image as base64 data URL.
                    let img_b64 = general_purpose::STANDARD.encode(&img_bytes);
                    let data_url = format!("data:{};base64,{}", mime, img_b64);

                    let mut convo: Vec<ChatMessage> = Vec::new();
                    // System prompt for the vision model.
                    convo.push(ChatMessage::new_text(Role::System, system_prompt));

                    for row in &history {
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

                    // Build multimodal message: text first, image second.
                    let vision_msg = ChatMessage {
                        role: Role::User,
                        content: MessageContent::Parts(vec![
                            MessagePart::Text {
                                text: user_prompt.to_string(),
                            },
                            MessagePart::ImageUrl {
                                image_url: ImageUrl::new(data_url),
                            },
                        ]),
                        tool_calls: None,
                        tool_call_id: None,
                    };

                    // Append the multimodal user message to the conversation.
                    convo.push(vision_msg);

                    // Send request to the vision model.
                    match groq
                        .chat(vision_model)
                        .messages(convo)
                        .max_completion_tokens(1200)
                        .temperature(0.2)
                        .send()
                        .await
                    {
                        Ok(vresp) => {
                            // Take the first model choice, if any.
                            if let Some(choice) = vresp.choices.first() {
                                // Try to extract plain text from structured content.
                                if let Some(text_out) =
                                    extract_text_from_message_content(&choice.message.content)
                                {
                                    image_section = format!(
                                        "Image analysis (vision model):\n{}\n\n",
                                        text_out.trim()
                                    );
                                } else {
                                    // Fallback: serialize raw response and search for strings.
                                    let raw = json!({
                                        "index": choice.index,
                                        "finish_reason": choice.finish_reason,
                                        "reasoning": choice.reasoning,
                                        "message": {
                                            "role": format!("{:?}", choice.message.role),
                                            "content": match &choice.message.content {
                                                MessageContent::Text(t) => json!(t),
                                                MessageContent::Parts(parts) => {
                                                    serde_json::to_value(parts).unwrap_or(json!(null))
                                                }
                                                _ => json!(null),
                                            }
                                        }
                                    });

                                    if let Some(found) = find_first_string_in_value(&raw) {
                                        image_section = format!(
                                            "Image analysis (vision model):\n{}\n\n",
                                            found.trim()
                                        );
                                    } else {
                                        image_section =
                                            "Image analysis: [no text extracted]\n\n".to_string();
                                    }
                                }
                            } else {
                                // No choices returned by the model.
                                image_section =
                                    "Image analysis: [no choices returned]\n\n".to_string();
                            }
                        }
                        Err(e) => {
                            // Vision model request failed.
                            error!("Vision model call failed: {}", e);
                            image_section = "Image analysis: [vision model error]\n\n".to_string();
                        }
                    }
                }
                Err(e) => {
                    // Image download failed.
                    error!("Failed downloading telegram image bytes: {}", e);
                }
            }
        } else {
            // Telegram file path could not be resolved.
            error!("Couldn't get file path from Telegram for photo.");
        }
    }

    image_section
}

// Returns the FileId of the largest available photo in the message.
fn largest_photo_file_id(msg: &Message) -> Option<FileId> {
    if let teloxide::types::MessageKind::Common(common) = &msg.kind
        && let teloxide::types::MediaKind::Photo(photo) = &common.media_kind
    {
        return photo
            .photo
            .iter()
            .max_by_key(|p| p.file.size)
            .map(|p| p.file.id.clone());
    }
    None
}

// Public helper: quickly check whether a message contains a photo.
// Use this to avoid calling analyze_image when there's no image at all.
pub fn message_has_photo(msg: &Message) -> bool {
    // Re-use the same pattern-match logic: return true if largest_photo_file_id yields Some.
    largest_photo_file_id(msg).is_some()
}

// Retrieves the remote Telegram file path for a given FileId.
async fn get_telegram_file_path(bot: &Bot, file_id: FileId) -> Option<String> {
    match bot.get_file(file_id).send().await {
        Ok(file) => Some(file.path),
        Err(e) => {
            error!("get_file error: {}", e);
            None
        }
    }
}

// Downloads a Telegram file directly into memory without disk I/O.
async fn download_telegram_file_bytes(
    bot: &Bot,
    file_path: &str,
) -> Result<Vec<u8>, reqwest::Error> {
    let token = bot.token().to_string();
    let url = format!("https://api.telegram.org/file/bot{}/{}", token, file_path);
    let resp = Client::new().get(&url).send().await?;
    let bytes = resp.bytes().await?;
    Ok(bytes.to_vec())
}

// Performs basic MIME type detection from file signatures.
fn detect_image_mime(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        "image/jpeg"
    } else if bytes.len() >= 8
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
        && bytes[4] == 0x0D
        && bytes[5] == 0x0A
        && bytes[6] == 0x1A
        && bytes[7] == 0x0A
    {
        "image/png"
    } else if bytes.len() >= 6 && (&bytes[0..6] == b"GIF89a" || &bytes[0..6] == b"GIF87a") {
        "image/gif"
    } else {
        "image/jpeg"
    }
}

// Recursively searches for the first non-empty string in arbitrary JSON.
fn find_first_string_in_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Array(arr) => arr.iter().find_map(find_first_string_in_value),
        Value::Object(map) => {
            for v in map.values() {
                if let Some(s) = find_first_string_in_value(v) {
                    return Some(s);
                }
            }
            None
        }
        _ => None,
    }
}

// Extracts usable text from a MessageContent structure.
fn extract_text_from_message_content(mc: &MessageContent) -> Option<String> {
    match mc {
        MessageContent::Text(s) => {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        MessageContent::Parts(parts) => {
            let mut out = String::new();
            for p in parts {
                if let MessagePart::Text { text } = p
                    && !text.trim().is_empty()
                {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(text.trim());
                }
            }
            if out.is_empty() { None } else { Some(out) }
        }
        _ => None,
    }
}
