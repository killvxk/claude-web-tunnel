#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 自动为子脚本添加可执行权限
chmod +x "$SCRIPT_DIR/scripts/"*.sh 2>/dev/null || true

source "$SCRIPT_DIR/scripts/utils.sh"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

show_banner() {
    echo -e "${CYAN}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                                                                ║"
    echo "║     Claude Web Tunnel - 远程 Claude Code 访问系统              ║"
    echo "║                                                                ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

show_help() {
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --server-only    仅安装 Server"
    echo "  --agent-only     仅安装 Agent"
    echo "  --skip-deps      跳过依赖安装 (MySQL/Redis/Nginx)"
    echo "  --dist-dir DIR   指定 dist 目录路径 (默认: ./dist)"
    echo "  --non-interactive  非交互模式 (使用默认值或命令行参数)"
    echo ""
    echo "非交互模式参数 (配合 --non-interactive 使用):"
    echo "  --nginx          安装 Nginx"
    echo "  --no-nginx       不安装 Nginx"
    echo "  --redis          安装 Redis"
    echo "  --no-redis       不安装 Redis"
    echo "  --mysql          使用 MySQL 数据库"
    echo "  --sqlite         使用 SQLite 数据库 (默认)"
    echo "  --tls            配置 Let's Encrypt TLS"
    echo "  --no-tls         不配置 TLS"
    echo "  --domain DOMAIN  指定域名"
    echo "  --email EMAIL    指定 Let's Encrypt 邮箱"
    echo "  --port PORT      指定 Server 端口 (默认 8080)"
    echo ""
    echo "  -h, --help       显示帮助"
    echo ""
    echo "示例:"
    echo "  sudo ./install.sh                        # 交互式安装"
    echo "  sudo ./install.sh --agent-only           # 仅安装 Agent (无需 root)"
    echo "  sudo ./install.sh --non-interactive --no-nginx --no-redis  # 最小化安装"
    echo "  sudo ./install.sh --non-interactive --nginx --redis --mysql --tls --domain example.com --email admin@example.com"
}

# 交互式询问函数 (默认 No)
ask_yes_no() {
    local prompt=$1
    local default=${2:-n}  # 默认为 n
    local result

    if [ "$default" = "y" ]; then
        read -p "$prompt [Y/n]: " result
        result=${result:-y}
    else
        read -p "$prompt [y/N]: " result
        result=${result:-n}
    fi

    case "$result" in
        [Yy]|[Yy][Ee][Ss]) return 0 ;;
        *) return 1 ;;
    esac
}

# 解析参数
INSTALL_SERVER=true
INSTALL_AGENT=false
SKIP_DEPS=false
DIST_DIR="$SCRIPT_DIR/../dist/linux"
NON_INTERACTIVE=false

# 可选组件 (默认值)
INSTALL_NGINX=""
INSTALL_REDIS=""
INSTALL_TLS=""
DB_TYPE=""
DOMAIN=""
LE_EMAIL=""
SERVER_PORT=8080

while [[ $# -gt 0 ]]; do
    case $1 in
        --server-only)
            INSTALL_SERVER=true
            INSTALL_AGENT=false
            shift
            ;;
        --agent-only)
            INSTALL_SERVER=false
            INSTALL_AGENT=true
            shift
            ;;
        --skip-deps)
            SKIP_DEPS=true
            shift
            ;;
        --dist-dir)
            DIST_DIR="$2"
            shift 2
            ;;
        --non-interactive)
            NON_INTERACTIVE=true
            shift
            ;;
        --nginx)
            INSTALL_NGINX=true
            shift
            ;;
        --no-nginx)
            INSTALL_NGINX=false
            shift
            ;;
        --redis)
            INSTALL_REDIS=true
            shift
            ;;
        --no-redis)
            INSTALL_REDIS=false
            shift
            ;;
        --mysql)
            DB_TYPE="mysql"
            shift
            ;;
        --sqlite)
            DB_TYPE="sqlite"
            shift
            ;;
        --tls)
            INSTALL_TLS=true
            shift
            ;;
        --no-tls)
            INSTALL_TLS=false
            shift
            ;;
        --domain)
            DOMAIN="$2"
            shift 2
            ;;
        --email)
            LE_EMAIL="$2"
            shift 2
            ;;
        --port)
            SERVER_PORT="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo -e "${RED}未知选项: $1${NC}"
            show_help
            exit 1
            ;;
    esac
done

# 检查 dist 目录
if [ ! -d "$DIST_DIR" ]; then
    echo -e "${RED}错误: dist 目录不存在: $DIST_DIR${NC}"
    echo "请先运行构建脚本生成二进制文件"
    exit 1
fi

export DIST_DIR

show_banner

# Agent 安装不需要 root
if [ "$INSTALL_AGENT" = true ] && [ "$INSTALL_SERVER" = false ]; then
    echo -e "${GREEN}=== 安装 Claude Tunnel Agent ===${NC}"
    echo ""
    "$SCRIPT_DIR/scripts/install-agent.sh"
    exit 0
fi

# Server 安装需要 root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Server 安装需要 root 权限${NC}"
    echo "用法: sudo ./install.sh"
    exit 1
fi

echo -e "${GREEN}=== Claude Web Tunnel 服务器部署 ===${NC}"
echo ""

# 检测系统
detect_os

# ============================================================
# 交互式配置 (智能默认)
# ============================================================

if [ "$NON_INTERACTIVE" = false ]; then
    echo -e "${CYAN}[配置] 组件选择${NC}"
    echo -e "${YELLOW}提示: 直接按回车使用默认值${NC}"
    echo ""

    # 1. 是否安装 Nginx (默认 No)
    if [ -z "$INSTALL_NGINX" ]; then
        if ask_yes_no "是否安装 Nginx 反向代理?"; then
            INSTALL_NGINX=true
        else
            INSTALL_NGINX=false
        fi
    fi

    # 2. 如果安装 Nginx，询问 TLS
    if [ "$INSTALL_NGINX" = true ]; then
        # 询问域名
        read -p "域名 (如 tunnel.example.com): " DOMAIN
        if [ -z "$DOMAIN" ]; then
            echo -e "${RED}使用 Nginx 必须提供域名${NC}"
            exit 1
        fi

        # 是否配置 TLS (默认 No)
        if [ -z "$INSTALL_TLS" ]; then
            if ask_yes_no "是否配置 Let's Encrypt TLS 证书?"; then
                INSTALL_TLS=true
            else
                INSTALL_TLS=false
            fi
        fi

        if [ "$INSTALL_TLS" = true ]; then
            read -p "Let's Encrypt 邮箱: " LE_EMAIL
            if [ -z "$LE_EMAIL" ]; then
                echo -e "${RED}TLS 配置需要提供邮箱${NC}"
                exit 1
            fi
        fi
    else
        INSTALL_TLS=false
        # 不使用 Nginx 时，可选择端口
        echo ""
        read -p "Server 监听端口 (默认 8080): " input_port
        SERVER_PORT=${input_port:-8080}
    fi

    echo ""

    # 3. 是否安装 Redis (默认 No)
    if [ -z "$INSTALL_REDIS" ]; then
        if ask_yes_no "是否安装 Redis (用于速率限制)?"; then
            INSTALL_REDIS=true
        else
            INSTALL_REDIS=false
        fi
    fi

    echo ""

    # 4. 数据库选择 (默认 SQLite)
    if [ -z "$DB_TYPE" ]; then
        echo -e "${CYAN}[配置] 数据库类型${NC}"
        echo "  S) SQLite (轻量，适合个人/测试) [默认]"
        echo "  m) MySQL (推荐生产使用)"
        read -p "选择 [S/m]: " db_choice
        db_choice=${db_choice:-S}

        case "$db_choice" in
            [Mm]) DB_TYPE="mysql" ;;
            *) DB_TYPE="sqlite" ;;
        esac
    fi
else
    # 非交互模式：设置默认值
    INSTALL_NGINX=${INSTALL_NGINX:-false}
    INSTALL_REDIS=${INSTALL_REDIS:-false}
    INSTALL_TLS=${INSTALL_TLS:-false}
    DB_TYPE=${DB_TYPE:-sqlite}

    # 验证必要参数
    if [ "$INSTALL_NGINX" = true ] && [ -z "$DOMAIN" ]; then
        echo -e "${RED}非交互模式下使用 Nginx 必须指定 --domain${NC}"
        exit 1
    fi

    if [ "$INSTALL_TLS" = true ] && [ -z "$LE_EMAIL" ]; then
        echo -e "${RED}非交互模式下使用 TLS 必须指定 --email${NC}"
        exit 1
    fi
fi

# MySQL 密码配置
MYSQL_ROOT_PASS=""
DB_PASS=""

if [ "$DB_TYPE" = "mysql" ]; then
    echo ""
    if [ "$NON_INTERACTIVE" = false ]; then
        read -p "MySQL root 密码: " -s MYSQL_ROOT_PASS
        echo
        if [ -z "$MYSQL_ROOT_PASS" ]; then
            echo -e "${RED}MySQL root 密码不能为空${NC}"
            exit 1
        fi

        read -p "应用数据库密码: " -s DB_PASS
        echo
        if [ -z "$DB_PASS" ]; then
            echo -e "${RED}应用数据库密码不能为空${NC}"
            exit 1
        fi
    else
        echo -e "${RED}非交互模式下 MySQL 需要手动配置密码${NC}"
        echo "请在安装后编辑: /opt/claude-tunnel/config/server.toml"
        DB_PASS="CHANGE_ME"
    fi
fi

# ============================================================
# 确认安装
# ============================================================

echo ""
echo -e "${YELLOW}即将安装以下组件:${NC}"
echo ""
echo "  核心组件:"
echo "    - Claude Tunnel Server"
if [ "$DB_TYPE" = "mysql" ]; then
    echo "    - 数据库: MySQL"
else
    echo "    - 数据库: SQLite"
fi
echo ""

if [ "$SKIP_DEPS" = false ]; then
    has_optional=false

    if [ "$INSTALL_NGINX" = true ] || [ "$INSTALL_REDIS" = true ]; then
        echo "  可选组件:"
        has_optional=true
    fi

    if [ "$INSTALL_REDIS" = true ]; then
        echo "    - Redis (速率限制)"
    fi

    if [ "$INSTALL_NGINX" = true ]; then
        if [ "$INSTALL_TLS" = true ]; then
            echo "    - Nginx + Let's Encrypt TLS"
        else
            echo "    - Nginx (无 TLS)"
        fi
    fi

    if [ "$has_optional" = false ]; then
        echo "  可选组件: 无"
    fi
fi

echo ""
echo "  配置:"
if [ "$INSTALL_NGINX" = true ]; then
    if [ "$INSTALL_TLS" = true ]; then
        echo "    - 访问地址: https://$DOMAIN"
    else
        echo "    - 访问地址: http://$DOMAIN"
    fi
else
    echo "    - 访问地址: http://<服务器IP>:$SERVER_PORT"
    echo "    - Server 直接绑定 0.0.0.0:$SERVER_PORT"
fi

if [ "$INSTALL_REDIS" = false ]; then
    echo "    - 速率限制: 禁用"
fi

echo ""
echo -e "${CYAN}二进制来源:${NC} $DIST_DIR"
echo ""

if [ "$NON_INTERACTIVE" = false ]; then
    read -p "确认继续? [y/N] " CONFIRM
    if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
        echo "安装已取消"
        exit 0
    fi
fi

# ============================================================
# 执行安装
# ============================================================

# 计算安装步骤数
STEP=1
TOTAL_STEPS=1  # Server 是必须的

if [ "$SKIP_DEPS" = false ]; then
    if [ "$INSTALL_REDIS" = true ]; then
        TOTAL_STEPS=$((TOTAL_STEPS + 1))
    fi
    if [ "$DB_TYPE" = "mysql" ]; then
        TOTAL_STEPS=$((TOTAL_STEPS + 1))
    fi
    if [ "$INSTALL_NGINX" = true ]; then
        TOTAL_STEPS=$((TOTAL_STEPS + 1))
    fi
fi

# 执行安装
if [ "$SKIP_DEPS" = false ]; then
    if [ "$INSTALL_REDIS" = true ]; then
        echo ""
        echo -e "${GREEN}[$STEP/$TOTAL_STEPS] 安装 Redis...${NC}"
        "$SCRIPT_DIR/scripts/install-redis.sh"
        STEP=$((STEP + 1))
    fi

    if [ "$DB_TYPE" = "mysql" ]; then
        echo ""
        echo -e "${GREEN}[$STEP/$TOTAL_STEPS] 安装 MySQL...${NC}"
        "$SCRIPT_DIR/scripts/install-mysql.sh" "$MYSQL_ROOT_PASS" "$DB_PASS"
        STEP=$((STEP + 1))
    fi
fi

echo ""
echo -e "${GREEN}[$STEP/$TOTAL_STEPS] 部署 Server...${NC}"

# 导出变量供子脚本使用
export INSTALL_NGINX
export INSTALL_REDIS
export SERVER_PORT

if [ "$DB_TYPE" = "mysql" ]; then
    "$SCRIPT_DIR/scripts/install-server.sh" "$DOMAIN" "mysql" "$DB_PASS"
else
    "$SCRIPT_DIR/scripts/install-server.sh" "$DOMAIN" "sqlite" ""
fi
STEP=$((STEP + 1))

if [ "$SKIP_DEPS" = false ] && [ "$INSTALL_NGINX" = true ]; then
    echo ""
    echo -e "${GREEN}[$STEP/$TOTAL_STEPS] 安装 Nginx...${NC}"
    if [ "$INSTALL_TLS" = true ]; then
        "$SCRIPT_DIR/scripts/install-nginx.sh" "$DOMAIN" "$LE_EMAIL"
    else
        "$SCRIPT_DIR/scripts/install-nginx.sh" "$DOMAIN" ""
    fi
fi

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗"
echo -e "║                      安装完成!                               ║"
echo -e "╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

if [ "$INSTALL_NGINX" = true ]; then
    if [ "$INSTALL_TLS" = true ]; then
        echo "访问地址: https://$DOMAIN"
    else
        echo "访问地址: http://$DOMAIN"
    fi
else
    echo "访问地址: http://<服务器IP>:$SERVER_PORT"
fi

echo ""
echo -e "${CYAN}服务管理命令:${NC}"
echo "  systemctl status claude-tunnel    # 查看状态"
echo "  systemctl restart claude-tunnel   # 重启服务"
echo "  journalctl -u claude-tunnel -f    # 查看日志"
echo ""

if [ "$INSTALL_NGINX" = true ] || [ "$INSTALL_REDIS" = true ] || [ "$DB_TYPE" = "mysql" ]; then
    echo -e "${CYAN}其他服务:${NC}"
    if [ "$INSTALL_NGINX" = true ]; then
        echo "  systemctl status nginx"
    fi
    if [ "$INSTALL_REDIS" = true ]; then
        echo "  systemctl status redis"
    fi
    if [ "$DB_TYPE" = "mysql" ]; then
        echo "  systemctl status mysql"
    fi
    echo ""
fi

echo -e "${YELLOW}重要: 请妥善保管上面输出的超级管理员 Token!${NC}"
