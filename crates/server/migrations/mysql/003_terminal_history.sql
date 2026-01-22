-- 终端历史记录表
-- 存储 PTY 输出块，用于用户重连时回放
CREATE TABLE IF NOT EXISTS terminal_history (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    instance_id VARCHAR(36) NOT NULL,
    sequence_number BIGINT NOT NULL,
    output_data MEDIUMTEXT NOT NULL,
    byte_size INT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY unique_instance_seq (instance_id, sequence_number)
);

-- 按实例 ID 和序列号索引，用于查询和清理
CREATE INDEX idx_terminal_history_instance_seq ON terminal_history(instance_id, sequence_number);

-- 按创建时间索引，用于清理过期数据
CREATE INDEX idx_terminal_history_created ON terminal_history(created_at);

-- 终端历史记录元数据表
-- 每个实例的缓冲区元数据
CREATE TABLE IF NOT EXISTS terminal_history_meta (
    instance_id VARCHAR(36) PRIMARY KEY,
    total_bytes BIGINT NOT NULL DEFAULT 0,
    next_sequence BIGINT NOT NULL DEFAULT 0,
    buffer_size_kb INT NOT NULL DEFAULT 64,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
