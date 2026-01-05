BEGIN;

-- =====================
-- TRIGGERS & FUNCTIONS
-- =====================
DROP TRIGGER IF EXISTS trg_soft_delete_chat ON chats;
DROP FUNCTION IF EXISTS soft_delete_chat_messages;

-- =====================
-- VIEWS
-- =====================
DROP VIEW IF EXISTS active_messages;
DROP VIEW IF EXISTS active_chats;
DROP VIEW IF EXISTS active_users;
DROP VIEW IF EXISTS active_languages;

-- =====================
-- TABLES (DEPENDENCY ORDER)
-- =====================
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS chats;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS languages;

COMMIT;
