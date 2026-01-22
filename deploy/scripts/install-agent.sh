#!/bin/bash
# Agent 安装脚本 (无需 root 权限)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

INSTALL_DIR="${HOME}/.claude-tunnel"

print_info "Claude Tunnel Agent 安装脚本"
echo ""

# 创建安装目录
mkdir -p "$INSTALL_DIR"/{bin,config,logs}

# 检测系统架构
detect_arch() {
    local arch=$(uname -m)
    case $arch in
        x86_64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            print_error "不支持的架构: $arch"
            exit 1
            ;;
    esac
}

# 检测操作系统
detect_os_type() {
    local os=$(uname -s)
    case $os in
        Linux)
            echo "linux"
            ;;
        Darwin)
            echo "macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "windows"
            ;;
        *)
            print_error "不支持的操作系统: $os"
            exit 1
            ;;
    esac
}

ARCH=$(detect_arch)
OS_TYPE=$(detect_os_type)

print_info "检测到系统: $OS_TYPE ($ARCH)"

# 根据操作系统设置 DIST_DIR
DIST_DIR="${DIST_DIR:-$SCRIPT_DIR/../../dist/$OS_TYPE}"

# 确定二进制文件名
if [ "$OS_TYPE" = "windows" ]; then
    BINARY_NAME="claude-tunnel-agent.exe"
else
    BINARY_NAME="claude-tunnel-agent"
fi

# 从 dist 目录获取二进制文件
get_binary() {
    local target_path="$INSTALL_DIR/bin/$BINARY_NAME"

    # 根据平台确定 dist 中的文件名
    local dist_binary_name
    if [ "$OS_TYPE" = "linux" ]; then
        dist_binary_name="claude-tunnel-agent-linux-$ARCH"
    elif [ "$OS_TYPE" = "macos" ]; then
        dist_binary_name="claude-tunnel-agent-macos-$ARCH"
    elif [ "$OS_TYPE" = "windows" ]; then
        dist_binary_name="claude-tunnel-agent-windows-$ARCH.exe"
    fi

    # 检查 dist 目录
    local search_paths=(
        "$DIST_DIR/$dist_binary_name"
        "$DIST_DIR/$BINARY_NAME"
        "$DIST_DIR/claude-tunnel-agent"
    )

    for path in "${search_paths[@]}"; do
        if [ -f "$path" ]; then
            print_info "复制二进制文件: $path"
            cp "$path" "$target_path"
            chmod +x "$target_path"
            return 0
        fi
    done

    # 备选：检查 target 目录 (开发环境)
    local target_binary
    if [ "$OS_TYPE" = "linux" ]; then
        target_binary="$SCRIPT_DIR/../../target/x86_64-unknown-linux-musl/release/claude-tunnel-agent"
    elif [ "$OS_TYPE" = "macos" ]; then
        if [ "$ARCH" = "aarch64" ]; then
            target_binary="$SCRIPT_DIR/../../target/aarch64-apple-darwin/release/claude-tunnel-agent"
        else
            target_binary="$SCRIPT_DIR/../../target/x86_64-apple-darwin/release/claude-tunnel-agent"
        fi
    elif [ "$OS_TYPE" = "windows" ]; then
        target_binary="$SCRIPT_DIR/../../target/release/claude-tunnel-agent.exe"
    fi

    if [ -f "$target_binary" ]; then
        print_info "从 target 目录复制: $target_binary"
        cp "$target_binary" "$target_path"
        chmod +x "$target_path"
        return 0
    fi

    print_error "未找到二进制文件"
    print_info "请先运行构建脚本生成 Agent 二进制"
    print_info "dist 目录: $DIST_DIR"
    return 1
}

# 获取 Agent 二进制
if ! get_binary; then
    print_warning "二进制文件获取失败"
    print_info "请手动下载或编译 Agent"
    exit 1
fi

# 交互式配置
echo ""
print_info "配置 Agent..."
echo ""

read -p "Agent 名称 (显示在 Web 界面): " AGENT_NAME
AGENT_NAME=${AGENT_NAME:-$(hostname)}

read -p "Server URL (如 wss://tunnel.example.com): " SERVER_URL
if [ -z "$SERVER_URL" ]; then
    print_error "Server URL 不能为空"
    exit 1
fi

# 询问是否已有 Token
echo ""
echo "Token 设置:"
echo "  1) 自动生成新 Token"
echo "  2) 使用已有 Token"
read -p "选择 [1/2] (默认 1): " TOKEN_CHOICE
TOKEN_CHOICE=${TOKEN_CHOICE:-1}

if [ "$TOKEN_CHOICE" = "2" ]; then
    read -p "Admin Token: " -s ADMIN_TOKEN
    echo
    read -p "Share Token (可选，回车跳过): " -s SHARE_TOKEN
    echo
else
    ADMIN_TOKEN=$(generate_random_string 32)
    SHARE_TOKEN=$(generate_random_string 32)

    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  生成的 Token (请妥善保管):                                  ║"
    echo "╠══════════════════════════════════════════════════════════════╣"
    echo "  Admin Token: $ADMIN_TOKEN"
    echo "  Share Token: $SHARE_TOKEN"
    echo "╚══════════════════════════════════════════════════════════════╝"
fi

# 生成配置文件
print_info "生成配置文件..."

cat > "$INSTALL_DIR/config/agent.toml" <<EOF
# Claude Tunnel Agent 配置
# 自动生成于 $(date '+%Y-%m-%d %H:%M:%S')

[agent]
name = "$AGENT_NAME"
admin_token = "$ADMIN_TOKEN"
share_token = "$SHARE_TOKEN"

[server]
url = "$SERVER_URL"
reconnect_interval = 5
heartbeat_interval = 30

[logging]
level = "info"
file = "$INSTALL_DIR/logs/agent.log"
rotation = "daily"
EOF

chmod 600 "$INSTALL_DIR/config/agent.toml"

# 创建启动脚本
cat > "$INSTALL_DIR/start.sh" <<EOF
#!/bin/bash
cd "$INSTALL_DIR"
exec ./bin/$BINARY_NAME --config ./config/agent.toml
EOF
chmod +x "$INSTALL_DIR/start.sh"

# Linux: 创建 systemd 用户服务
if [ "$OS_TYPE" = "linux" ]; then
    print_info "创建 systemd 用户服务..."

    mkdir -p "$HOME/.config/systemd/user"

    cat > "$HOME/.config/systemd/user/claude-tunnel-agent.service" <<EOF
[Unit]
Description=Claude Tunnel Agent
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/bin/$BINARY_NAME --config $INSTALL_DIR/config/agent.toml
Restart=always
RestartSec=5
WorkingDirectory=$INSTALL_DIR
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

    systemctl --user daemon-reload

    echo ""
    print_info "启动 Agent 服务:"
    echo "  systemctl --user enable --now claude-tunnel-agent"
    echo ""
    print_info "查看日志:"
    echo "  journalctl --user -u claude-tunnel-agent -f"
fi

# macOS: 创建 launchd 服务
if [ "$OS_TYPE" = "macos" ]; then
    print_info "创建 launchd 服务..."

    mkdir -p "$HOME/Library/LaunchAgents"

    cat > "$HOME/Library/LaunchAgents/com.claude-tunnel.agent.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.claude-tunnel.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/bin/$BINARY_NAME</string>
        <string>--config</string>
        <string>$INSTALL_DIR/config/agent.toml</string>
    </array>
    <key>WorkingDirectory</key>
    <string>$INSTALL_DIR</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$INSTALL_DIR/logs/agent.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$INSTALL_DIR/logs/agent.stderr.log</string>
</dict>
</plist>
EOF

    echo ""
    print_info "启动 Agent 服务:"
    echo "  launchctl load ~/Library/LaunchAgents/com.claude-tunnel.agent.plist"
    echo ""
    print_info "停止服务:"
    echo "  launchctl unload ~/Library/LaunchAgents/com.claude-tunnel.agent.plist"
fi

# Windows: 提示手动启动
if [ "$OS_TYPE" = "windows" ]; then
    echo ""
    print_info "Windows 启动方式:"
    echo "  手动运行: $INSTALL_DIR/start.sh"
    echo "  或直接运行: $INSTALL_DIR/bin/$BINARY_NAME --config $INSTALL_DIR/config/agent.toml"
    echo ""
    print_info "建议使用 Task Scheduler 设置开机自启"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                    Agent 安装完成!                           ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
print_info "安装目录: $INSTALL_DIR"
print_info "配置文件: $INSTALL_DIR/config/agent.toml"
print_info "日志文件: $INSTALL_DIR/logs/agent.log"
echo ""

if [ "$TOKEN_CHOICE" = "1" ]; then
    print_warning "请妥善保管上面显示的 Token!"
    print_info "Admin Token 用于管理权限，Share Token 用于共享访问"
fi
