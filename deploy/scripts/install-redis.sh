#!/bin/bash
# Redis 安装脚本
# 支持: Ubuntu/Debian, CentOS/RHEL

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

detect_os

print_info "安装 Redis..."

if [ "$OS" = "debian" ]; then
    $PKG_UPDATE
    $PKG_INSTALL redis-server
    REDIS_CONF="/etc/redis/redis.conf"
elif [ "$OS" = "redhat" ]; then
    $PKG_INSTALL epel-release
    $PKG_INSTALL redis
    REDIS_CONF="/etc/redis.conf"
fi

print_info "配置 Redis..."

# 备份原配置
cp "$REDIS_CONF" "${REDIS_CONF}.bak"

# 配置 Redis (仅本地访问)
sed -i 's/^bind .*/bind 127.0.0.1/' "$REDIS_CONF"
sed -i 's/^# maxmemory .*/maxmemory 256mb/' "$REDIS_CONF"
sed -i 's/^# maxmemory-policy .*/maxmemory-policy allkeys-lru/' "$REDIS_CONF"

# 启动服务
systemctl enable --now redis-server 2>/dev/null || systemctl enable --now redis

# 等待服务启动
sleep 2

# 测试连接
if redis-cli ping | grep -q "PONG"; then
    print_success "Redis 安装完成并运行正常"
else
    print_error "Redis 安装完成但连接测试失败"
    exit 1
fi
