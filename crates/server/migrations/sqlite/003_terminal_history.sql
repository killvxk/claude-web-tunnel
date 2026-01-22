-- 终端历史记录表
-- 存储 PTY 输出块，用于用户重连时回放
CREATE TABLE IF NOT EXISTS terminal_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL,
    sequence_number INTEGER NOT NULL,
    output_data TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(instance_id, sequence_number)
);

-- 按实例 ID 和序列号索引，用于查询和清理
CREATE INDEX IF NOT EXISTS idx_terminal_history_instance_seq ON terminal_history(instance_id, sequence_number);

-- 按创建时间索引，用于清理过期数据
CREATE INDEX IF NOT EXISTS idx_terminal_history_created ON terminal_history(created_at);

-- 终端历史记录元数据表
-- 每个实例的缓冲区元数据
CREATE TABLE IF NOT EXISTS terminal_history_meta (
    instance_id TEXT PRIMARY KEY,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    next_sequence INTEGER NOT NULL DEFAULT 0,
    buffer_size_kb INTEGER NOT NULL DEFAULT 64,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
