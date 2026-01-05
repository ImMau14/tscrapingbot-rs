CREATE OR REPLACE FUNCTION public.get_recent_messages(
  p_lang TEXT,
  p_user_telegram_id BIGINT,
  p_chat_telegram_id BIGINT,
  p_limit INT
)
RETURNS TABLE(content TEXT, ia_response TEXT)
LANGUAGE sql
AS $$
WITH
  ins_language AS (
    INSERT INTO languages (name)
    VALUES (p_lang)
    ON CONFLICT DO NOTHING
    RETURNING id
  ),
  language AS (
    SELECT id FROM ins_language
    UNION ALL
    SELECT id
    FROM languages
    WHERE name = p_lang
      AND deleted_at IS NULL
    LIMIT 1
  ),
  ins_user AS (
    INSERT INTO users (telegram_id, lang_id)
    SELECT p_user_telegram_id, id FROM language
    ON CONFLICT DO NOTHING
  ),
  ins_chat AS (
    INSERT INTO chats (telegram_id)
    VALUES (p_chat_telegram_id)
    ON CONFLICT DO NOTHING
  ),
  msgs AS (
    SELECT
      m.content,
      m.ia_response
    FROM messages m
    WHERE m.user_telegram_id = p_user_telegram_id
      AND m.chat_telegram_id = p_chat_telegram_id
      AND m.deleted_at IS NULL
      AND m.is_cleared = FALSE
    ORDER BY m.created_at DESC
    LIMIT p_limit
  )
SELECT content, ia_response FROM msgs
UNION ALL
SELECT NULL, NULL
WHERE NOT EXISTS (SELECT 1 FROM msgs);
$$;
