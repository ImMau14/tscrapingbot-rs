BEGIN;

-- =====================
-- LANGUAGES
-- =====================
CREATE TABLE languages (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name VARCHAR NOT NULL,
  deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX uq_languages_name_active
ON languages(name)
WHERE deleted_at IS NULL;

-- =====================
-- USERS
-- =====================
CREATE TABLE users (
  telegram_id BIGINT PRIMARY KEY,
  lang_id BIGINT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ,

  CONSTRAINT fk_users_lang
    FOREIGN KEY (lang_id)
    REFERENCES languages(id),

  CONSTRAINT chk_users_deleted_after_created
    CHECK (deleted_at IS NULL OR deleted_at >= created_at)
);

CREATE INDEX idx_users_lang_id
ON users(lang_id)
WHERE deleted_at IS NULL;

-- =====================
-- CHATS
-- =====================
CREATE TABLE chats (
  telegram_id BIGINT PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ,

  CONSTRAINT chk_chats_deleted_after_created
    CHECK (deleted_at IS NULL OR deleted_at >= created_at)
);

-- =====================
-- MESSAGES
-- =====================
CREATE TABLE messages (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_telegram_id BIGINT NOT NULL,
  chat_telegram_id BIGINT NOT NULL,
  content TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ia_response TEXT,
  is_cleared BOOLEAN NOT NULL DEFAULT false,
  deleted_at TIMESTAMPTZ,

  CONSTRAINT fk_messages_user
    FOREIGN KEY (user_telegram_id)
    REFERENCES users(telegram_id),

  CONSTRAINT fk_messages_chat
    FOREIGN KEY (chat_telegram_id)
    REFERENCES chats(telegram_id),

  CONSTRAINT chk_messages_deleted_after_created
    CHECK (deleted_at IS NULL OR deleted_at >= created_at)
);

CREATE INDEX idx_messages_user_telegram_id
ON messages(user_telegram_id)
WHERE deleted_at IS NULL;

CREATE INDEX idx_messages_chat_telegram_id
ON messages(chat_telegram_id)
WHERE deleted_at IS NULL;

-- =====================
-- VIEWS (ACTIVE ENTITIES)
-- =====================
CREATE VIEW active_languages AS
SELECT *
FROM languages
WHERE deleted_at IS NULL;

CREATE VIEW active_users AS
SELECT *
FROM users
WHERE deleted_at IS NULL;

CREATE VIEW active_chats AS
SELECT *
FROM chats
WHERE deleted_at IS NULL;

CREATE VIEW active_messages AS
SELECT *
FROM messages
WHERE deleted_at IS NULL;

-- =====================
-- SOFT DELETE CASCADE (CHAT → MESSAGES)
-- =====================
CREATE OR REPLACE FUNCTION soft_delete_chat_messages()
RETURNS trigger AS $$
BEGIN
  UPDATE messages
  SET deleted_at = now()
  WHERE chat_telegram_id = OLD.telegram_id
    AND deleted_at IS NULL;

  RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_soft_delete_chat
AFTER UPDATE OF deleted_at ON chats
FOR EACH ROW
WHEN (OLD.deleted_at IS NULL AND NEW.deleted_at IS NOT NULL)
EXECUTE FUNCTION soft_delete_chat_messages();

COMMIT;
