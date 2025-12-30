-- Chat types
CREATE TABLE IF NOT EXISTS chat_types (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name VARCHAR NOT NULL UNIQUE
);

-- Languages
CREATE TABLE IF NOT EXISTS languages (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name VARCHAR NOT NULL UNIQUE
);

-- Users
CREATE TABLE IF NOT EXISTS users (
  telegram_id BIGINT PRIMARY KEY,
  lang_id BIGINT REFERENCES languages(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Chats
CREATE TABLE IF NOT EXISTS chats (
  telegram_id BIGINT PRIMARY KEY,
  chat_type_id BIGINT REFERENCES chat_types(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Messages
CREATE TABLE IF NOT EXISTS messages (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_telegram_id BIGINT NOT NULL,
  chat_telegram_id BIGINT,
  content TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ia_response TEXT,
  is_cleared BOOLEAN NOT NULL DEFAULT false,

  CONSTRAINT fk_messages_user
    FOREIGN KEY (user_telegram_id)
    REFERENCES users(telegram_id)
    ON DELETE CASCADE,

  CONSTRAINT fk_messages_chat
    FOREIGN KEY (chat_telegram_id)
    REFERENCES chats(telegram_id)
    ON DELETE SET NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_users_lang_id
  ON users(lang_id);

CREATE INDEX IF NOT EXISTS idx_chats_chat_type_id
  ON chats(chat_type_id);

CREATE INDEX IF NOT EXISTS idx_messages_user_telegram_id
  ON messages(user_telegram_id);

CREATE INDEX IF NOT EXISTS idx_messages_chat_telegram_id
  ON messages(chat_telegram_id);
