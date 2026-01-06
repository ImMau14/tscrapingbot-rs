// Keep message/user extraction logic in one place so handlers stay thin.

use teloxide::types::Message;

// Extract and normalize core user/chat identifiers from a Telegram `Message`.
pub fn extract_user_info(msg: &Message) -> Result<(i64, String, i64), String> {
    // Get the sender object; if absent we can't proceed
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| "The user could not be identified.".to_string())?;

    // Normalize id and language with sensible defaults
    let user_id: i64 = user.id.0 as i64;
    let user_lang = user
        .language_code
        .clone()
        .unwrap_or_else(|| "en".to_string());

    // If message is inside a forum thread, prefer thread id (keeps history grouped)
    let msg_chat_id: i64 = msg
        .thread_id
        .map(|tid| tid.0.0 as i64)
        .unwrap_or(msg.chat.id.0);

    Ok((user_id, user_lang, msg_chat_id))
}
