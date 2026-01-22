-- Agents table stores registered agents and their tokens
CREATE TABLE IF NOT EXISTS agents (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    admin_token_hash VARCHAR(64) NOT NULL UNIQUE,
    share_token_hash VARCHAR(64) NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_connected_at DATETIME NULL
);

-- Index for faster token lookup during authentication
CREATE INDEX idx_agents_admin_token ON agents(admin_token_hash);
CREATE INDEX idx_agents_share_token ON agents(share_token_hash);
