#!/bin/bash
# Nginx + TLS (Let's Encrypt) 安装脚本
# 支持: Ubuntu/Debian, CentOS/RHEL
# 如果 LE_EMAIL 为空，则跳过 TLS 配置

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

DOMAIN=$1
LE_EMAIL=$2

if [ -z "$DOMAIN" ]; then
    print_error "用法: $0 <domain> [email]"
    print_info "如果不提供 email，则跳过 Let's Encrypt TLS 配置"
    exit 1
fi

# 判断是否启用 TLS
ENABLE_TLS=false
if [ -n "$LE_EMAIL" ]; then
    ENABLE_TLS=true
fi

detect_os

print_info "安装 Nginx..."

if [ "$OS" = "debian" ]; then
    $PKG_UPDATE
    if [ "$ENABLE_TLS" = true ]; then
        $PKG_INSTALL nginx certbot python3-certbot-nginx
    else
        $PKG_INSTALL nginx
    fi
elif [ "$OS" = "redhat" ]; then
    install_epel
    if [ "$ENABLE_TLS" = true ]; then
        $PKG_INSTALL nginx certbot python3-certbot-nginx
    else
        $PKG_INSTALL nginx
    fi
fi

# 确保 sites-available 和 sites-enabled 目录存在
mkdir -p /etc/nginx/sites-available
mkdir -p /etc/nginx/sites-enabled

# 检查 nginx.conf 是否包含 sites-enabled
if ! grep -q "sites-enabled" /etc/nginx/nginx.conf; then
    # 在 http 块末尾添加 include
    sed -i '/http {/a \    include /etc/nginx/sites-enabled/*;' /etc/nginx/nginx.conf
fi

if [ "$ENABLE_TLS" = true ]; then
    # ========== TLS 模式 ==========
    print_info "创建初始 Nginx 配置 (用于证书申请)..."

    # 创建初始配置 (HTTP only, 用于证书申请)
    cat > /etc/nginx/sites-available/claude-tunnel <<EOF
server {
    listen 80;
    server_name $DOMAIN;

    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }

    location / {
        return 301 https://\$server_name\$request_uri;
    }
}
EOF

    ln -sf /etc/nginx/sites-available/claude-tunnel /etc/nginx/sites-enabled/

    # 删除默认站点
    rm -f /etc/nginx/sites-enabled/default

    # 测试并启动 Nginx
    nginx -t
    systemctl enable --now nginx

    print_info "申请 Let's Encrypt 证书..."

    # 申请证书
    certbot certonly --nginx -d "$DOMAIN" --non-interactive --agree-tos -m "$LE_EMAIL"

    print_info "更新 Nginx 配置 (HTTPS + WebSocket)..."

    # 更新为完整 HTTPS 配置
    cat > /etc/nginx/sites-available/claude-tunnel <<EOF
# HTTP -> HTTPS 重定向
server {
    listen 80;
    server_name $DOMAIN;

    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }

    location / {
        return 301 https://\$server_name\$request_uri;
    }
}

# HTTPS 主配置
server {
    listen 443 ssl http2;
    server_name $DOMAIN;

    # TLS 证书 (Let's Encrypt)
    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;
    ssl_trusted_certificate /etc/letsencrypt/live/$DOMAIN/chain.pem;

    # TLS 优化
    ssl_session_timeout 1d;
    ssl_session_cache shared:SSL:50m;
    ssl_session_tickets off;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # HSTS
    add_header Strict-Transport-Security "max-age=63072000" always;

    # 静态资源 + API
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # WebSocket 代理 (子端连接)
    location /ws/agent {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }

    # WebSocket 代理 (用户连接)
    location /ws/user {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }
}
EOF

    # 设置证书自动续期
    print_info "设置证书自动续期..."
    echo "0 0,12 * * * root certbot renew --quiet --post-hook 'systemctl reload nginx'" > /etc/cron.d/certbot-renew
    chmod 644 /etc/cron.d/certbot-renew

    print_success "Nginx + TLS 安装完成"
    print_info "域名: https://$DOMAIN"
    print_info "证书将自动续期"

else
    # ========== 无 TLS 模式 ==========
    print_info "创建 Nginx 配置 (HTTP only)..."

    # 创建纯 HTTP 配置
    cat > /etc/nginx/sites-available/claude-tunnel <<EOF
# HTTP 配置 (无 TLS)
server {
    listen 80;
    server_name $DOMAIN;

    # 静态资源 + API
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # WebSocket 代理 (子端连接)
    location /ws/agent {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }

    # WebSocket 代理 (用户连接)
    location /ws/user {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }
}
EOF

    ln -sf /etc/nginx/sites-available/claude-tunnel /etc/nginx/sites-enabled/

    # 删除默认站点
    rm -f /etc/nginx/sites-enabled/default

    print_success "Nginx 安装完成 (无 TLS)"
    print_info "域名: http://$DOMAIN"
    print_warning "警告: 未启用 TLS，连接不安全！建议在生产环境中启用 HTTPS"
fi

# 测试并重载配置
nginx -t
systemctl enable --now nginx
systemctl reload nginx
