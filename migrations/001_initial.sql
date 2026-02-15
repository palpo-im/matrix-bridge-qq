CREATE TABLE IF NOT EXISTS portal (
    chat_type TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    room_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL DEFAULT '',
    created_at BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (chat_type, chat_id)
);

CREATE TABLE IF NOT EXISTS message_map (
    source TEXT NOT NULL,
    source_msg_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    chat_type TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    matrix_event_id TEXT,
    qq_message_id TEXT,
    created_at BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (source, source_msg_id)
);

CREATE TABLE IF NOT EXISTS processed_txn (
    txn_id TEXT PRIMARY KEY,
    processed_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS qq_user (
    qq_user_id TEXT PRIMARY KEY,
    mxid TEXT NOT NULL UNIQUE,
    displayname TEXT NOT NULL DEFAULT '',
    avatar_url TEXT,
    updated_at BIGINT NOT NULL DEFAULT 0
);
