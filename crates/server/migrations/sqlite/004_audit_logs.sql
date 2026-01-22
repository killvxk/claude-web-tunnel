-- Audit logs table for SEC-002
-- Stores user operation audit trail

CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    event_type TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_role TEXT NOT NULL,
    agent_id TEXT,
    instance_id TEXT,
    target_id TEXT,
    client_ip TEXT NOT NULL,
    success INTEGER NOT NULL DEFAULT 1,
    details TEXT
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_session ON audit_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_agent ON audit_logs(agent_id);
