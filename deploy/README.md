# Claude Web Tunnel 部署指南

## 快速开始

### Server 部署 (Linux)

```bash
# 下载部署包
curl -sL https://github.com/yourname/claude-web-tunnel/releases/latest/download/deploy.tar.gz | tar xz
cd deploy

# 运行安装 (需要 root 权限)
sudo ./install.sh
```

### Agent 安装 (多平台)

```bash
# Linux/macOS (无需 root)
./install.sh --agent-only

# 或使用本地编译的二进制
./install.sh --agent-only --local
```

## 系统要求

### Server

- **操作系统**: Ubuntu 20.04+, Debian 10+, CentOS 7+, RHEL 7+
- **内存**: 最低 512MB，推荐 1GB+
- **存储**: 最低 1GB 可用空间
- **网络**: 需要公网 IP 和域名

### Agent

- **操作系统**: Linux, macOS, Windows
- **架构**: x86_64, aarch64 (ARM64)
- **网络**: 能够访问 Server

## 安装选项

```bash
# 完整服务器安装 (Server + 依赖)
sudo ./install.sh

# 仅安装 Server (跳过 MySQL/Redis/Nginx)
sudo ./install.sh --server-only --skip-deps

# 仅安装 Agent
./install.sh --agent-only

# 使用本地编译的二进制文件
sudo ./install.sh --local

# 指定版本
sudo ./install.sh --version v0.1.0
```

## 安装内容

### Server 组件

1. **Redis** - 速率限制和实时状态
2. **MySQL** (可选) - 持久化存储，可选择 SQLite
3. **Nginx** - 反向代理 + Let's Encrypt TLS
4. **Claude Tunnel Server** - 主服务 (内嵌 Web 前端)

### Agent 组件

1. **Claude Tunnel Agent** - 本地代理
2. **配置文件** - agent.toml
3. **系统服务** - systemd (Linux) / launchd (macOS)

## 目录结构

### Server

```
/opt/claude-tunnel/
├── bin/
│   └── claude-tunnel-server    # 主程序 (内嵌 Web 前端)
├── config/
│   └── server.toml             # 配置文件
├── data/
│   └── tunnel.db               # SQLite 数据库 (如使用)
└── logs/
    └── server.log              # 日志文件 (每日轮转)
```

### Agent

```
~/.claude-tunnel/
├── bin/
│   └── claude-tunnel-agent     # Agent 程序
├── config/
│   └── agent.toml              # 配置文件
└── logs/
    └── agent.log               # 日志文件 (每日轮转)
```

## 服务管理

### Server (systemd)

```bash
# 启动/停止/重启
sudo systemctl start claude-tunnel
sudo systemctl stop claude-tunnel
sudo systemctl restart claude-tunnel

# 查看状态和日志
sudo systemctl status claude-tunnel
sudo journalctl -u claude-tunnel -f
```

### Agent (Linux - systemd user)

```bash
# 启动/停止
systemctl --user start claude-tunnel-agent
systemctl --user stop claude-tunnel-agent

# 开机自启
systemctl --user enable claude-tunnel-agent

# 查看日志
journalctl --user -u claude-tunnel-agent -f
```

### Agent (macOS - launchd)

```bash
# 启动
launchctl load ~/Library/LaunchAgents/com.claude-tunnel.agent.plist

# 停止
launchctl unload ~/Library/LaunchAgents/com.claude-tunnel.agent.plist
```

## 配置说明

### Server 配置 (server.toml)

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
type = "sqlite"                 # 或 "mysql"
sqlite_path = "/opt/claude-tunnel/data/tunnel.db"
# mysql_url = "mysql://user:pass@localhost/claude_tunnel"
redis_url = "redis://127.0.0.1:6379"

[security]
super_admin_token = "your-token"
rate_limit_per_minute = 10
token_min_length = 32

[logging]
level = "info"
file = "/opt/claude-tunnel/logs/server.log"
rotation = "daily"              # 或 "hourly"

[terminal_history]
enabled = true                  # 启用终端历史回放
default_buffer_size_kb = 64     # 默认缓冲区大小 (KB)
max_buffer_size_kb = 512        # 最大缓冲区大小 (KB)
retention_days = 7              # 历史记录保留天数

[audit_log]
enabled = true                  # 启用审计日志
retention_days = 30             # 审计日志保留天数
```

#### 配置节说明

| 配置节 | 说明 |
|-------|------|
| `[server]` | HTTP 服务器监听配置 |
| `[database]` | 数据库连接配置，支持 SQLite/MySQL + Redis |
| `[security]` | 安全配置，包括超管 Token 和速率限制 |
| `[logging]` | 日志配置，支持文件轮转 |
| `[terminal_history]` | 终端历史回放，断线重连后恢复之前的输出内容 |
| `[audit_log]` | 审计日志，记录用户操作用于安全审计 |

#### 审计日志记录的事件

- 认证成功/失败
- 创建/关闭终端实例
- 附加/分离终端会话
- 管理员操作（强制断开 Agent、删除 Agent、强制关闭实例）
- 标签操作（添加/移除 Agent 标签）

### Agent 配置 (agent.toml)

```toml
[agent]
name = "my-workstation"
admin_token = "your-admin-token"
share_token = "your-share-token"

[server]
url = "wss://tunnel.example.com"
reconnect_interval = 5
heartbeat_interval = 30

[logging]
level = "info"
file = "./logs/agent.log"
rotation = "daily"
```

## Token 说明

| Token 类型 | 权限 |
|-----------|------|
| 超级管理员 Token | Server 配置的 super_admin_token，可管理所有 Agent |
| Admin Token | Agent 的完整权限（创建/关闭实例、选择目录、操作终端） |
| Share Token | Agent 的只读+操作权限（查看、操作现有实例） |

## TLS 证书

证书由 Let's Encrypt 自动申请和续期。

手动续期:
```bash
sudo certbot renew
sudo systemctl reload nginx
```

## 构建二进制

如需自行编译：

```bash
# Linux (musl 静态链接)
./build-linux.sh

# Windows (PowerShell)
.\build-windows.ps1

# macOS (支持 universal binary)
./build-macos.sh --universal

# 输出目录: ./dist/
```

## 故障排查

### Server 无法启动

```bash
# 检查日志
sudo journalctl -u claude-tunnel -n 50

# 检查配置文件
cat /opt/claude-tunnel/config/server.toml

# 检查端口占用
sudo ss -tlnp | grep 8080
```

### Agent 无法连接

```bash
# 检查网络连通性
curl -I https://tunnel.example.com/health

# 检查 Agent 日志
cat ~/.claude-tunnel/logs/agent.log

# 检查 Token 是否正确
grep token ~/.claude-tunnel/config/agent.toml
```

### Redis 连接失败

```bash
redis-cli ping
sudo systemctl status redis
```

### MySQL 连接失败

```bash
mysql -u tunnel -p -e "SELECT 1"
sudo systemctl status mysql
```

## 卸载

### Server

```bash
sudo systemctl stop claude-tunnel
sudo systemctl disable claude-tunnel
sudo rm -rf /opt/claude-tunnel
sudo rm /etc/systemd/system/claude-tunnel.service
sudo systemctl daemon-reload
```

### Agent (Linux)

```bash
systemctl --user stop claude-tunnel-agent
systemctl --user disable claude-tunnel-agent
rm -rf ~/.claude-tunnel
rm ~/.config/systemd/user/claude-tunnel-agent.service
```

### Agent (macOS)

```bash
launchctl unload ~/Library/LaunchAgents/com.claude-tunnel.agent.plist
rm -rf ~/.claude-tunnel
rm ~/Library/LaunchAgents/com.claude-tunnel.agent.plist
```

## 安全建议

1. 使用强随机 Token（至少 32 字符）
2. 定期轮换 Token
3. 限制 SSH 访问
4. 启用防火墙，只开放必要端口 (80, 443)
5. 定期备份数据库
6. 监控异常登录尝试
7. 启用审计日志 (`audit_log.enabled = true`) 并定期审查
8. 根据合规要求调整审计日志保留期限
