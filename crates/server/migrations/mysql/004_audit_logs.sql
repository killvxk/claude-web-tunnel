-- Audit logs table for SEC-002
-- Stores user operation audit trail

CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    event_type VARCHAR(50) NOT NULL,
    session_id VARCHAR(36) NOT NULL,
    user_role VARCHAR(20) NOT NULL,
    agent_id VARCHAR(36),
    instance_id VARCHAR(36),
    target_id VARCHAR(36),
    client_ip VARCHAR(45) NOT NULL,
    success TINYINT NOT NULL DEFAULT 1,
    details TEXT,

    INDEX idx_audit_logs_timestamp (timestamp),
    INDEX idx_audit_logs_event_type (event_type),
    INDEX idx_audit_logs_session (session_id),
    INDEX idx_audit_logs_agent (agent_id)
);
