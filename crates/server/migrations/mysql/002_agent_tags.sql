-- Agent 标签表
-- 每个 Agent 可以有多个标签，用于分组显示
CREATE TABLE IF NOT EXISTS agent_tags (
    agent_id VARCHAR(36) NOT NULL,
    tag VARCHAR(100) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (agent_id, tag),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- 按标签名称索引，用于获取所有标签
CREATE INDEX idx_agent_tags_tag ON agent_tags(tag);
