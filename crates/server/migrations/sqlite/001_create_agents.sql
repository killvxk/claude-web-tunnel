-- Agents table stores registered agents and their tokens
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    admin_token_hash TEXT NOT NULL UNIQUE,
    share_token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_connected_at TEXT
);

-- Index for faster token lookup during authentication
CREATE INDEX IF NOT EXISTS idx_agents_admin_token ON agents(admin_token_hash);
CREATE INDEX IF NOT EXISTS idx_agents_share_token ON agents(share_token_hash);
