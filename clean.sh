#!/bin/bash
# 清理脚本 - 删除构建产物和依赖

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
echo_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Parse arguments
CLEAN_DEPS=false
CLEAN_TARGET=false
CLEAN_DIST=false
CLEAN_ALL=false

show_help() {
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --deps       清理 node_modules"
    echo "  --target     清理 Rust target 目录"
    echo "  --dist       清理 dist 输出目录"
    echo "  --all        清理所有 (deps + target + dist)"
    echo "  -h, --help   显示帮助"
    echo ""
    echo "示例:"
    echo "  ./clean.sh --deps          # 只清理 node_modules"
    echo "  ./clean.sh --all           # 清理所有"
}

if [ $# -eq 0 ]; then
    show_help
    exit 0
fi

while [[ $# -gt 0 ]]; do
    case $1 in
        --deps)
            CLEAN_DEPS=true
            shift
            ;;
        --target)
            CLEAN_TARGET=true
            shift
            ;;
        --dist)
            CLEAN_DIST=true
            shift
            ;;
        --all)
            CLEAN_ALL=true
            shift
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

if [ "$CLEAN_ALL" = true ]; then
    CLEAN_DEPS=true
    CLEAN_TARGET=true
    CLEAN_DIST=true
fi

# Clean node_modules
if [ "$CLEAN_DEPS" = true ]; then
    if [ -d "web/node_modules" ]; then
        echo_info "删除 web/node_modules..."
        rm -rf web/node_modules
        echo_info "web/node_modules 已删除"
    else
        echo_warn "web/node_modules 不存在"
    fi
fi

# Clean target
if [ "$CLEAN_TARGET" = true ]; then
    if [ -d "target" ]; then
        echo_info "删除 target/..."
        rm -rf target
        echo_info "target/ 已删除"
    else
        echo_warn "target/ 不存在"
    fi
fi

# Clean dist
if [ "$CLEAN_DIST" = true ]; then
    if [ -d "dist" ]; then
        echo_info "删除 dist/..."
        rm -rf dist
        echo_info "dist/ 已删除"
    else
        echo_warn "dist/ 不存在"
    fi
fi

echo_info "清理完成!"
