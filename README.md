# Claude Web Tunnel

通过 Web 界面远程操作本地 Claude Code 实例的隧道系统。

## 功能特性

### 核心功能
- **远程访问** - 通过浏览器远程操作本地 Claude Code
- **多实例支持** - 同一 Agent 可运行多个 Claude Code 实例
- **多人协作** - 团队成员可共享同一终端会话
- **终端历史回放** - 断线重连后自动恢复之前的输出内容
- **审计日志** - 记录用户操作，支持安全审计

### 权限控制
| Token 类型 | 权限范围 |
|-----------|---------|
| 超级管理员 | 管理所有 Agent，选择任意 Agent 管理其实例，强制断开/删除 Agent，查看全局统计和审计日志 |
| Admin Token | 创建/关闭实例、选择目录、操作终端 |
| Share Token | 查看和操作现有实例（只读权限） |

### 前端特性
- **主题切换** - 支持 Dark/Light 主题
- **移动端适配** - 响应式布局，支持手机/平板
- **PWA 支持** - 可安装为桌面应用，支持离线缓存
- **实例搜索** - 多关键词模糊搜索，搜索历史
- **标签分组** - 按标签或目录对实例进行分组管理
- **管理面板** - 超级管理员可查看所有 Agent 和实例

## 架构

```
                                    ┌─────────────────────────────────┐
                                    │         远程服务器              │
┌─────────────┐                     │  ┌─────────────────────────┐   │
│  本地 Agent │ ────── WSS ──────▶ │  │    Claude Tunnel Server │   │
│             │                     │  │  (内嵌 Web 前端)        │   │
│ Claude Code │                     │  └───────────┬─────────────┘   │
└─────────────┘                     │              │                 │
                                    │     SQLite/MySQL + Redis       │
                                    └──────────────┼─────────────────┘
                                                   │
                                    ┌──────────────┴──────────────┐
                                    │     Nginx (可选)            │
                                    │  HTTPS + Let's Encrypt      │
                                    └──────────────┬──────────────┘
                                                   │
                                    ◀────── HTTPS/WSS ──────▶

                            ┌───────────────┐  ┌───────────────┐
                            │  Web 浏览器   │  │   移动设备    │
                            └───────────────┘  └───────────────┘
```

## 快速开始

### 1. 服务器部署 (Linux)

```bash
# 1. 克隆仓库
git clone https://github.com/yourname/claude-web-tunnel.git
cd claude-web-tunnel

# 2. 构建（自动安装 Rust 和 Node.js）
./build-linux.sh

# 3. 部署
cd deploy
sudo ./install.sh
```

安装脚本会交互式询问配置选项：
- 是否安装 Nginx 反向代理（推荐，用于 HTTPS）
- 是否安装 Redis（用于速率限制，防暴力破解）
- 数据库类型（SQLite 适合小规模，MySQL 适合大规模）
- 是否配置 Let's Encrypt TLS（免费 HTTPS 证书）

安装完成后，脚本会显示生成的 **SuperAdmin Token**，请妥善保存。

### 2. Agent 安装与连接

Agent 运行在你的本地机器上，通过 WebSocket 连接到远程服务器。

#### 2.1 获取 Agent 二进制文件

```bash
# 方式一：从构建产物复制
scp your-server:/path/to/dist/linux/claude-tunnel-agent ./

# 方式二：本地构建
./build-linux.sh --agent-only
```

#### 2.2 创建配置文件

创建 `agent.toml` 配置文件：

```toml
[agent]
name = "my-workstation"           # Agent 显示名称（在 Web 界面显示）
admin_token = "your-admin-token"  # 管理员 Token（自己设定，32+ 字符）
share_token = "your-share-token"  # 分享 Token（自己设定，32+ 字符）

[server]
url = "https://tunnel.example.com"  # 服务器地址（支持 http/https/ws/wss）
reconnect_interval = 5              # 断线重连间隔（秒）
heartbeat_interval = 30             # 心跳间隔（秒）

[logging]
level = "info"
file = "./logs/agent.log"
rotation = "daily"
```

> **注意**：`admin_token` 和 `share_token` 由你自己设定，不是服务器生成的。每个 Agent 可以有不同的 Token。

#### 2.3 启动 Agent

```bash
# 前台运行（调试用）
./claude-tunnel-agent --config agent.toml

# 后台运行（生产用）
nohup ./claude-tunnel-agent --config agent.toml > /dev/null 2>&1 &

# 或使用 systemd（推荐）
systemctl --user start claude-tunnel-agent
systemctl --user enable claude-tunnel-agent  # 开机自启
```

#### 2.4 连接流程图

```
┌─────────────────┐         ┌─────────────────────┐         ┌─────────────────┐
│   本地 Agent    │         │    远程 Server      │         │   Web 浏览器    │
│                 │         │                     │         │                 │
│ agent.toml:     │  WSS    │  验证 admin_token   │  HTTPS  │  输入 Token     │
│ - admin_token ──┼────────▶│  或 share_token     │◀────────┼── 登录         │
│ - share_token   │         │                     │         │                 │
│ - server_url    │         │  Agent 注册成功     │         │  显示终端列表   │
└─────────────────┘         └─────────────────────┘         └─────────────────┘
```

### 3. Web 界面使用

访问服务器地址（如 `https://tunnel.example.com`），输入 Token 登录。

---

## Token 系统详解

系统使用三种 Token 进行权限控制：

### Token 类型对比

| Token 类型 | 设定方式 | 权限范围 | 使用场景 |
|-----------|---------|---------|---------|
| **SuperAdmin Token** | 服务器 `server.toml` 配置 | 管理所有 Agent，查看全局统计，强制操作 | 系统管理员 |
| **Admin Token** | Agent `agent.toml` 配置 | 创建/关闭实例，选择目录，完全操作终端 | Agent 所有者 |
| **Share Token** | Agent `agent.toml` 配置 | 查看和操作现有实例（不能创建新实例） | 团队协作成员 |

### 详细说明

#### SuperAdmin Token（超级管理员）

**配置位置**：服务器端 `server.toml`

```toml
[security]
super_admin_token = "your-super-secret-token-at-least-32-chars"
```

**权限**：
- ✅ 查看所有在线 Agent 和实例
- ✅ **选择任意 Agent 进行实例管理**（创建/关闭实例、操作终端）
- ✅ 强制断开任意 Agent
- ✅ 强制关闭任意实例
- ✅ 删除 Agent 记录
- ✅ 查看全局统计信息
- ✅ 查看审计日志
- ✅ 管理 Agent 标签

**使用场景**：系统运维管理员，需要全局管控能力。

**工作流程**：
1. 使用 SuperAdmin Token 登录后自动进入 Admin Panel
2. 在 Agent 列表中点击在线 Agent 的"选择"按钮
3. 跳转到 Instances 页面，可创建/管理该 Agent 的实例
4. 页面顶部显示当前管理的 Agent 名称
5. 点击"返回 Agent 列表"可切换到其他 Agent

---

#### Admin Token（管理员）

**配置位置**：Agent 端 `agent.toml`

```toml
[agent]
admin_token = "your-admin-token-at-least-32-chars"
```

**权限**：
- ✅ 创建新的终端实例
- ✅ 选择实例工作目录
- ✅ 关闭实例
- ✅ 操作终端（输入命令）
- ✅ 调整终端大小
- ❌ 不能管理其他 Agent

**使用场景**：Agent 所有者，拥有该 Agent 的完全控制权。

**登录方式**：在 Web 界面输入 Admin Token 登录。

---

#### Share Token（分享）

**配置位置**：Agent 端 `agent.toml`

```toml
[agent]
share_token = "your-share-token-at-least-32-chars"
```

**权限**：
- ✅ 查看现有实例列表
- ✅ 附加到现有实例（共享终端）
- ✅ 操作终端（输入命令）
- ❌ 不能创建新实例
- ❌ 不能关闭实例
- ❌ 不能选择目录

**使用场景**：团队协作，将 Share Token 分享给同事，让他们可以查看和操作你的终端。

**登录方式**：在 Web 界面输入 Share Token 登录。

---

### Token 生成建议

使用强随机字符串（至少 32 字符）：

```bash
# Linux/macOS
openssl rand -base64 32

# 或使用 Python
python3 -c "import secrets; print(secrets.token_urlsafe(32))"
```

### 多 Agent 场景

每个 Agent 可以有不同的 Token：

```
Agent A (办公电脑):
  admin_token = "office-admin-xxx..."
  share_token = "office-share-xxx..."

Agent B (家用电脑):
  admin_token = "home-admin-xxx..."
  share_token = "home-share-xxx..."
```

用户使用对应的 Token 登录，只能看到和操作对应的 Agent。

---

## 服务器部署详解

### 部署架构选项

#### 方案一：完整部署（推荐生产环境）

```
互联网 ──▶ Nginx (443) ──▶ Claude Tunnel Server (8080) ──▶ SQLite/MySQL
                │                                              │
                └── Let's Encrypt TLS                   Redis (可选)
```

```bash
sudo ./install.sh
# 选择：Nginx ✓, Redis ✓, TLS ✓
```

#### 方案二：最小化部署（测试/个人使用）

```
互联网 ──▶ Claude Tunnel Server (8080) ──▶ SQLite
```

```bash
sudo ./install.sh --no-nginx --no-redis
# Server 直接绑定 0.0.0.0:8080
```

### 手动部署步骤

如果不使用安装脚本，可以手动部署：

#### 1. 准备目录

```bash
sudo mkdir -p /opt/claude-tunnel/{bin,config,data,logs}
sudo cp dist/linux/claude-tunnel-server /opt/claude-tunnel/bin/
sudo chmod +x /opt/claude-tunnel/bin/claude-tunnel-server
```

#### 2. 创建配置文件

```bash
sudo nano /opt/claude-tunnel/config/server.toml
```

```toml
[server]
host = "127.0.0.1"    # Nginx 代理时用 127.0.0.1
port = 8080

[database]
db_type = "sqlite"
sqlite_path = "/opt/claude-tunnel/data/tunnel.db"
# redis_url = "redis://127.0.0.1:6379"  # 可选

[security]
super_admin_token = "your-super-admin-token-here"  # 必须修改！
rate_limit_per_minute = 10
token_min_length = 32

[logging]
level = "info"
file = "/opt/claude-tunnel/logs/server.log"
rotation = "daily"

[terminal_history]
enabled = true
default_buffer_size_kb = 64
max_buffer_size_kb = 512
retention_days = 7

[audit_log]
enabled = true
retention_days = 30
```

#### 3. 创建 systemd 服务

```bash
sudo nano /etc/systemd/system/claude-tunnel.service
```

```ini
[Unit]
Description=Claude Web Tunnel Server
After=network.target

[Service]
Type=simple
User=claude-tunnel
Group=claude-tunnel
WorkingDirectory=/opt/claude-tunnel
ExecStart=/opt/claude-tunnel/bin/claude-tunnel-server --config /opt/claude-tunnel/config/server.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

#### 4. 创建用户并启动服务

```bash
sudo useradd -r -s /bin/false claude-tunnel
sudo chown -R claude-tunnel:claude-tunnel /opt/claude-tunnel
sudo systemctl daemon-reload
sudo systemctl enable claude-tunnel
sudo systemctl start claude-tunnel
```

#### 5. 配置 Nginx（可选但推荐）

```bash
sudo nano /etc/nginx/sites-available/claude-tunnel
```

```nginx
server {
    listen 80;
    server_name tunnel.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name tunnel.example.com;

    ssl_certificate /etc/letsencrypt/live/tunnel.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/tunnel.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;  # WebSocket 长连接
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/claude-tunnel /etc/nginx/sites-enabled/
sudo certbot --nginx -d tunnel.example.com  # 申请 TLS 证书
sudo systemctl reload nginx
```

### 验证部署

```bash
# 检查服务状态
sudo systemctl status claude-tunnel

# 检查健康端点
curl http://127.0.0.1:8080/health

# 检查日志
sudo tail -f /opt/claude-tunnel/logs/server.log

# 检查端口
sudo ss -tlnp | grep 8080
```

## 项目结构

```
claude-web-tunnel/
├── crates/
│   ├── common/              # 共享类型库
│   │   ├── config.rs        # 配置类型定义
│   │   ├── protocol.rs      # WebSocket 协议消息
│   │   ├── types.rs         # 核心数据类型
│   │   └── error.rs         # 错误类型
│   ├── agent/               # 本地 Agent CLI
│   │   ├── connection.rs    # WebSocket 连接管理
│   │   ├── pty.rs           # PTY 实例管理
│   │   └── instance.rs      # 多实例管理器
│   └── server/              # 远程服务器
│       ├── routes.rs        # HTTP/WS 路由
│       ├── state.rs         # 应用状态管理
│       ├── ws_agent.rs      # Agent 连接处理
│       ├── ws_user.rs       # 用户连接处理
│       ├── db/              # 数据库层
│       │   ├── repository.rs # 数据访问层
│       │   └── schema.rs    # 数据模型
│       ├── rate_limit.rs    # Redis 速率限制
│       └── static_files.rs  # 嵌入式静态文件服务
├── web/                     # Svelte 5 前端
│   ├── src/
│   │   ├── lib/             # 组件 (Terminal, Login, AdminPanel 等)
│   │   ├── stores/          # Svelte stores (auth, theme, tags 等)
│   │   └── services/        # WebSocket 服务
│   └── vite.config.ts       # Vite + PWA 配置
├── deploy/                  # 部署脚本
│   ├── install.sh           # 主安装脚本
│   ├── scripts/             # 子安装脚本
│   └── configs/             # 配置文件模板
├── build-linux.sh           # Linux musl 静态构建
├── build-windows.ps1        # Windows 构建
├── build-macos.sh           # macOS 构建 (支持 universal binary)
└── clean.sh / clean.ps1     # 清理脚本
```

## 技术栈

| 层面 | 技术 |
|------|------|
| **后端** | Rust (Axum, Tokio, SQLx) |
| **前端** | Svelte 5 + xterm.js + Tailwind CSS |
| **数据库** | SQLite / MySQL + Redis (可选) |
| **通信** | WebSocket (JSON 消息) |
| **部署** | Nginx + Let's Encrypt (可选) |

## 开发

### 环境要求

- Rust 1.75+
- Node.js 20+
- pnpm (推荐) 或 npm

### 本地运行

```bash
# 1. 启动服务器
cargo run -p server -- --config server.toml

# 2. 启动前端开发服务器 (另一个终端)
cd web
pnpm install
pnpm dev

# 3. 启动 Agent (另一个终端)
cargo run -p agent -- --config agent.toml
```

### 运行测试

```bash
cargo test --workspace
```

## 配置

### 服务器配置 (server.toml)

```toml
[server]
host = "127.0.0.1"          # 监听地址 (Nginx 代理时用 127.0.0.1，直连时用 0.0.0.0)
port = 8080                 # 监听端口

[database]
db_type = "sqlite"          # 数据库类型: "sqlite" 或 "mysql"
sqlite_path = "./data/tunnel.db"
# mysql_url = "mysql://user:pass@localhost/claude_tunnel"
# redis_url = "redis://127.0.0.1:6379"  # 可选，用于速率限制

[security]
super_admin_token = "YOUR_SECRET_TOKEN"  # 超级管理员 Token (必须修改!)
rate_limit_per_minute = 10               # 每分钟最大认证尝试
token_min_length = 32                    # Token 最小长度

[logging]
level = "info"              # 日志级别: trace/debug/info/warn/error
file = "./logs/server.log"  # 日志文件路径
rotation = "daily"          # 日志轮转: daily/hourly

[terminal_history]
enabled = true              # 启用终端历史回放
default_buffer_size_kb = 64 # 默认缓冲区大小 (KB)
max_buffer_size_kb = 512    # 最大缓冲区大小 (KB)
retention_days = 7          # 历史记录保留天数

[audit_log]
enabled = true              # 启用审计日志
retention_days = 30         # 审计日志保留天数
```

### Agent 配置 (agent.toml)

```toml
[agent]
name = "my-workstation"           # Agent 显示名称
admin_token = "your-admin-token"  # 管理员 Token
share_token = "your-share-token"  # 分享 Token

[server]
url = "wss://tunnel.example.com"  # 服务器地址 (不含 /ws/agent 后缀)
reconnect_interval = 5            # 重连间隔 (秒)
heartbeat_interval = 30           # 心跳间隔 (秒)

[logging]
level = "info"
file = "./logs/agent.log"
rotation = "daily"
```

### 配置节说明

| 配置节 | 说明 |
|-------|------|
| `[server]` | HTTP 服务器监听配置 |
| `[database]` | 数据库连接配置，支持 SQLite/MySQL + Redis |
| `[security]` | 安全配置，包括超管 Token 和速率限制 |
| `[logging]` | 日志配置，支持每日/每小时轮转 |
| `[terminal_history]` | 终端历史回放配置，用于断线重连后恢复输出 |
| `[audit_log]` | 审计日志配置，记录用户操作用于安全审计 |

## 构建

### Linux (musl 静态链接)

```bash
./build-linux.sh                # 构建 server 和 agent
./build-linux.sh --server-only  # 仅构建 server
./build-linux.sh --agent-only   # 仅构建 agent
./build-linux.sh --skip-deps    # 跳过依赖安装
```

输出目录: `dist/linux/`

### Windows (PowerShell)

```powershell
.\build-windows.ps1
.\build-windows.ps1 -ServerOnly
.\build-windows.ps1 -AgentOnly
```

输出目录: `dist/windows/`

### macOS

```bash
./build-macos.sh                # 构建当前架构
./build-macos.sh --universal    # 构建 universal binary (x86_64 + arm64)
```

输出目录: `dist/macos/`

## 服务管理

### Server (systemd)

```bash
sudo systemctl start claude-tunnel
sudo systemctl stop claude-tunnel
sudo systemctl restart claude-tunnel
sudo systemctl status claude-tunnel
sudo journalctl -u claude-tunnel -f
```

### Agent (Linux - systemd user)

```bash
systemctl --user start claude-tunnel-agent
systemctl --user stop claude-tunnel-agent
systemctl --user enable claude-tunnel-agent  # 开机自启
journalctl --user -u claude-tunnel-agent -f
```

### Agent (macOS - launchd)

```bash
launchctl load ~/Library/LaunchAgents/com.claude-tunnel.agent.plist
launchctl unload ~/Library/LaunchAgents/com.claude-tunnel.agent.plist
```

## 审计日志

启用 `[audit_log]` 后，系统会记录以下操作：

| 事件类型 | 说明 |
|---------|------|
| `auth_success` / `auth_failure` | 认证成功/失败 |
| `create_instance` / `close_instance` | 创建/关闭终端实例 |
| `attach` / `detach` | 附加/分离终端会话 |
| `force_disconnect_agent` | SuperAdmin 强制断开 Agent |
| `force_close_instance` | SuperAdmin 强制关闭实例 |
| `delete_agent` | SuperAdmin 删除 Agent |
| `select_working_agent` | SuperAdmin 选择工作 Agent |
| `clear_working_agent` | SuperAdmin 清除工作 Agent |
| `add_agent_tag` / `remove_agent_tag` | 标签操作 |

审计日志存储在数据库的 `audit_logs` 表中，系统每小时自动清理超过保留期限的记录。

## 安全建议

1. 使用强随机 Token（至少 32 字符）
2. 启用审计日志并定期审查
3. 定期轮换 Token
4. 使用 HTTPS (启用 Nginx + Let's Encrypt)
5. 启用 Redis 速率限制防止暴力破解
6. 限制 SSH 访问
7. 启用防火墙，只开放必要端口 (80, 443)
8. 定期备份数据库

## 故障排查

### Server 无法启动

```bash
sudo journalctl -u claude-tunnel -n 50
cat /opt/claude-tunnel/config/server.toml
sudo ss -tlnp | grep 8080
```

### Agent 无法连接

```bash
curl -I https://tunnel.example.com/health
cat ~/.claude-tunnel/logs/agent.log
```

### Redis 连接失败

```bash
redis-cli ping
sudo systemctl status redis
```

## 未实现功能

以下功能尚在规划中：

| 功能 | 优先级 | 状态 |
|------|--------|------|
| 目录白名单安全限制 | 中 | 暂缓 |
| Token 过期机制 | 中 | 暂缓 |
| Docker 镜像 | 低 | 待定 |
| 文件传输 | 中 | 待定 |
| 终端录制/分享 | 低 | 待定 |

完整清单见 `docs/memory/backlog.md`

## License

MIT License
