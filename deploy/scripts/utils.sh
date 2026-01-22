#!/bin/bash
# 通用工具函数
# 兼容 CentOS 7+, Debian 10+, Ubuntu 20.04+

# 检测操作系统
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS_NAME="$NAME"
        OS_VERSION="$VERSION_ID"
    fi

    if [ -f /etc/debian_version ]; then
        OS="debian"
        PKG_MGR="apt"
        PKG_UPDATE="apt update -y"
        PKG_INSTALL="apt install -y"
    elif [ -f /etc/redhat-release ]; then
        OS="redhat"
        # CentOS 7 uses yum, CentOS 8+ and Fedora use dnf
        if command -v dnf >/dev/null 2>&1; then
            PKG_MGR="dnf"
            PKG_UPDATE="dnf update -y"
            PKG_INSTALL="dnf install -y"
        else
            PKG_MGR="yum"
            PKG_UPDATE="yum update -y"
            PKG_INSTALL="yum install -y"
        fi
    elif [ -f /etc/alpine-release ]; then
        OS="alpine"
        PKG_MGR="apk"
        PKG_UPDATE="apk update"
        PKG_INSTALL="apk add"
    else
        echo "不支持的操作系统"
        exit 1
    fi
    export OS PKG_MGR PKG_UPDATE PKG_INSTALL OS_NAME OS_VERSION
}

# 检查命令是否存在
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# 检查服务是否运行
service_running() {
    systemctl is-active --quiet "$1" 2>/dev/null
}

# 等待服务启动
wait_for_service() {
    local service="$1"
    local max_wait="${2:-30}"
    local count=0

    while [ "$count" -lt "$max_wait" ]; do
        if service_running "$service"; then
            return 0
        fi
        sleep 1
        count=$((count + 1))
    done
    return 1
}

# 生成随机字符串 (兼容 CentOS 7)
generate_random_string() {
    local length="${1:-32}"
    if command_exists openssl; then
        openssl rand -base64 "$length" | tr -d '/+=' | head -c "$length"
    elif [ -f /dev/urandom ]; then
        head -c 100 /dev/urandom | base64 | tr -d '/+=' | head -c "$length"
    else
        # Fallback: 使用日期和进程ID
        echo "$(date +%s%N)$$" | sha256sum | head -c "$length"
    fi
}

# 打印带颜色的消息
print_info() {
    echo -e "\033[0;34m[INFO]\033[0m $1"
}

print_success() {
    echo -e "\033[0;32m[SUCCESS]\033[0m $1"
}

print_warning() {
    echo -e "\033[1;33m[WARNING]\033[0m $1"
}

print_error() {
    echo -e "\033[0;31m[ERROR]\033[0m $1"
}

# 安装 EPEL (CentOS/RHEL)
install_epel() {
    if [ "$OS" = "redhat" ]; then
        if ! rpm -q epel-release >/dev/null 2>&1; then
            print_info "安装 EPEL 仓库..."
            $PKG_INSTALL epel-release
        fi
    fi
}
