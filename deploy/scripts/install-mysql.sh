#!/bin/bash
# MySQL 安装脚本
# 支持: Ubuntu/Debian, CentOS/RHEL

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

MYSQL_ROOT_PASS=$1
DB_PASS=$2
DB_NAME="claude_tunnel"
DB_USER="tunnel"

if [ -z "$MYSQL_ROOT_PASS" ] || [ -z "$DB_PASS" ]; then
    print_error "用法: $0 <mysql_root_password> <app_db_password>"
    exit 1
fi

detect_os

print_info "安装 MySQL..."

if [ "$OS" = "debian" ]; then
    # Debian/Ubuntu
    export DEBIAN_FRONTEND=noninteractive

    # 预设 root 密码
    debconf-set-selections <<< "mysql-server mysql-server/root_password password $MYSQL_ROOT_PASS"
    debconf-set-selections <<< "mysql-server mysql-server/root_password_again password $MYSQL_ROOT_PASS"

    $PKG_UPDATE
    $PKG_INSTALL mysql-server mysql-client

elif [ "$OS" = "redhat" ]; then
    # CentOS/RHEL
    $PKG_INSTALL mysql-server mysql

    # 启动服务
    systemctl enable --now mysqld

    # 获取临时密码并修改
    TEMP_PASS=$(grep 'temporary password' /var/log/mysqld.log | awk '{print $NF}' | tail -1)

    if [ -n "$TEMP_PASS" ]; then
        mysql --connect-expired-password -u root -p"$TEMP_PASS" <<EOF
ALTER USER 'root'@'localhost' IDENTIFIED BY '$MYSQL_ROOT_PASS';
EOF
    fi
fi

# 启动服务
systemctl enable --now mysql 2>/dev/null || systemctl enable --now mysqld

print_info "配置数据库..."

# 创建数据库和用户
mysql -u root -p"$MYSQL_ROOT_PASS" <<EOF
CREATE DATABASE IF NOT EXISTS $DB_NAME CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER IF NOT EXISTS '$DB_USER'@'localhost' IDENTIFIED BY '$DB_PASS';
GRANT ALL PRIVILEGES ON $DB_NAME.* TO '$DB_USER'@'localhost';
FLUSH PRIVILEGES;
EOF

# 初始化表结构
mysql -u root -p"$MYSQL_ROOT_PASS" $DB_NAME < "$SCRIPT_DIR/../configs/mysql.sql"

print_success "MySQL 安装完成"
print_info "数据库: $DB_NAME"
print_info "用户: $DB_USER"
