#!/bin/bash
# Server 部署脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

DOMAIN=$1
DB_TYPE=$2  # "sqlite" 或 "mysql"
DB_PASS=$3
INSTALL_DIR="/opt/claude-tunnel"

# 从父脚本继承的环境变量
DIST_DIR="${DIST_DIR:-$SCRIPT_DIR/../../dist/linux}"
INSTALL_NGINX="${INSTALL_NGINX:-true}"
INSTALL_REDIS="${INSTALL_REDIS:-true}"
SERVER_PORT="${SERVER_PORT:-8080}"

if [ -z "$DB_TYPE" ]; then
    print_error "用法: $0 <domain> <sqlite|mysql> [db_password]"
    exit 1
fi

if [ "$DB_TYPE" = "mysql" ] && [ -z "$DB_PASS" ]; then
    print_error "MySQL 模式需要提供数据库密码"
    exit 1
fi

# 根据是否使用 Nginx 决定绑定地址
if [ "$INSTALL_NGINX" = "true" ]; then
    SERVER_HOST="127.0.0.1"
else
    SERVER_HOST="0.0.0.0"
fi

print_info "创建安装目录..."

# 创建目录
mkdir -p "$INSTALL_DIR"/{bin,config,logs,data}

# 从 dist 目录获取二进制文件
get_binary() {
    local binary_name=$1
    local target_path=$2

    # 检查 dist 目录中的二进制文件
    local binary_path="$DIST_DIR/$binary_name"

    if [ -f "$binary_path" ]; then
        print_info "复制二进制文件: $binary_path"
        cp "$binary_path" "$target_path"
        chmod +x "$target_path"
        return 0
    fi

    # 备选：检查 target 目录 (开发环境)
    local target_binary="$SCRIPT_DIR/../../target/x86_64-unknown-linux-musl/release/$binary_name"
    if [ -f "$target_binary" ]; then
        print_info "从 target 目录复制: $target_binary"
        cp "$target_binary" "$target_path"
        chmod +x "$target_path"
        return 0
    fi

    print_error "未找到二进制文件: $binary_name"
    print_info "请先运行构建脚本: ./build-linux.sh"
    return 1
}

# 获取 Server 二进制
if ! get_binary "claude-tunnel-server" "$INSTALL_DIR/bin/claude-tunnel-server"; then
    print_warning "二进制文件获取失败"
    print_info "请手动编译并复制到: $INSTALL_DIR/bin/claude-tunnel-server"
    print_info "编译命令: ./build-linux.sh --server-only"
fi

print_info "生成配置文件..."

# 生成超级管理员 Token
SUPER_ADMIN_TOKEN=$(generate_random_string 32)

# 生成 Redis 配置（可选）
if [ "$INSTALL_REDIS" = "true" ]; then
    REDIS_CONFIG='redis_url = "redis://127.0.0.1:6379"'
else
    REDIS_CONFIG='# redis_url = "redis://127.0.0.1:6379"  # Redis 未安装，速率限制已禁用'
fi

# 生成配置文件
if [ "$DB_TYPE" = "mysql" ]; then
    cat > "$INSTALL_DIR/config/server.toml" <<EOF
# Claude Web Tunnel Server 配置
# 自动生成于 $(date '+%Y-%m-%d %H:%M:%S')

[server]
host = "$SERVER_HOST"
port = $SERVER_PORT

[database]
type = "mysql"
mysql_url = "mysql://tunnel:$DB_PASS@localhost/claude_tunnel"
$REDIS_CONFIG

[security]
super_admin_token = "$SUPER_ADMIN_TOKEN"
rate_limit_per_minute = 10
token_min_length = 32

[logging]
level = "info"
file = "$INSTALL_DIR/logs/server.log"
rotation = "daily"

[terminal_history]
enabled = true
default_buffer_size_kb = 64
max_buffer_size_kb = 512
retention_days = 7

[audit_log]
enabled = true
retention_days = 30
EOF
else
    cat > "$INSTALL_DIR/config/server.toml" <<EOF
# Claude Web Tunnel Server 配置
# 自动生成于 $(date '+%Y-%m-%d %H:%M:%S')

[server]
host = "$SERVER_HOST"
port = $SERVER_PORT

[database]
type = "sqlite"
sqlite_path = "$INSTALL_DIR/data/tunnel.db"
$REDIS_CONFIG

[security]
super_admin_token = "$SUPER_ADMIN_TOKEN"
rate_limit_per_minute = 10
token_min_length = 32

[logging]
level = "info"
file = "$INSTALL_DIR/logs/server.log"
rotation = "daily"

[terminal_history]
enabled = true
default_buffer_size_kb = 64
max_buffer_size_kb = 512
retention_days = 7

[audit_log]
enabled = true
retention_days = 30
EOF
fi

print_info "创建 systemd 服务..."

# 根据是否安装 Redis 设置 systemd 依赖
if [ "$INSTALL_REDIS" = "true" ]; then
    SYSTEMD_AFTER="After=network.target redis.service"
    SYSTEMD_WANTS="Wants=redis.service"
else
    SYSTEMD_AFTER="After=network.target"
    SYSTEMD_WANTS=""
fi

# 创建 systemd 服务
cat > /etc/systemd/system/claude-tunnel.service <<EOF
[Unit]
Description=Claude Web Tunnel Server
$SYSTEMD_AFTER
$SYSTEMD_WANTS

[Service]
Type=simple
User=www-data
Group=www-data
ExecStart=$INSTALL_DIR/bin/claude-tunnel-server --config $INSTALL_DIR/config/server.toml
Restart=always
RestartSec=5
WorkingDirectory=$INSTALL_DIR
Environment=RUST_LOG=info

# 安全加固
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_DIR/logs $INSTALL_DIR/data
PrivateTmp=true

# 资源限制
LimitNOFILE=65535
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

print_info "设置权限..."

# 创建 www-data 用户 (如果不存在)
id -u www-data &>/dev/null || useradd -r -s /sbin/nologin www-data

# 设置权限
chown -R www-data:www-data "$INSTALL_DIR"
chmod 700 "$INSTALL_DIR/config"
chmod 600 "$INSTALL_DIR/config/server.toml"

# 重载 systemd
systemctl daemon-reload

# 启动服务 (如果二进制文件存在)
if [ -f "$INSTALL_DIR/bin/claude-tunnel-server" ]; then
    print_info "启动服务..."
    systemctl enable --now claude-tunnel

    # 等待服务启动
    sleep 2

    if systemctl is-active --quiet claude-tunnel; then
        print_success "Server 部署完成并已启动"
    else
        print_warning "服务启动失败，请检查日志: journalctl -u claude-tunnel"
    fi
else
    print_info "服务已配置，待二进制文件就绪后执行:"
    print_info "  systemctl enable --now claude-tunnel"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  超级管理员 Token (请妥善保管):                              ║"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "  $SUPER_ADMIN_TOKEN"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

print_info "配置文件: $INSTALL_DIR/config/server.toml"
print_info "日志文件: $INSTALL_DIR/logs/server.log"
print_info "数据目录: $INSTALL_DIR/data/"
print_info "监听地址: $SERVER_HOST:$SERVER_PORT"
if [ "$INSTALL_NGINX" != "true" ]; then
    print_warning "Server 直接绑定公网，请确保防火墙配置正确"
fi
if [ "$INSTALL_REDIS" != "true" ]; then
    print_warning "未启用 Redis，速率限制功能已禁用"
fi
