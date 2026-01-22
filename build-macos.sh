#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case $ARCH in
        x86_64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "aarch64"
            ;;
        *)
            echo "$ARCH"
            ;;
    esac
}

# Function to generate hash file
generate_hash() {
    local binary=$1
    local hash_file=$2

    echo_info "Generating hash file: $hash_file"

    # Get current date
    local build_date=$(date -u '+%Y-%m-%d %H:%M:%S UTC')

    # Calculate hashes
    local sha256=$(shasum -a 256 "$binary" | awk '{print $1}')
    local md5=$(md5 -q "$binary" 2>/dev/null || md5sum "$binary" | awk '{print $1}')
    local size=$(du -h "$binary" | cut -f1)
    local size_bytes=$(stat -f%z "$binary" 2>/dev/null || stat -c%s "$binary" 2>/dev/null)

    # Write hash file
    cat > "$hash_file" << EOF
# Claude Web Tunnel Build Hash
# Generated: $build_date

File: $(basename "$binary")
Size: $size ($size_bytes bytes)

SHA256: $sha256
MD5:    $md5
EOF

    echo_info "Hash file generated successfully"
}

# Main script
echo "========================================"
echo "  Claude Web Tunnel macOS Build Script"
echo "========================================"
echo ""

# Parse arguments
BUILD_SERVER=true
BUILD_AGENT=true
SKIP_WEB=false
UNIVERSAL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --server-only)
            BUILD_AGENT=false
            shift
            ;;
        --agent-only)
            BUILD_SERVER=false
            shift
            ;;
        --skip-web)
            SKIP_WEB=true
            shift
            ;;
        --universal)
            UNIVERSAL=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --server-only    Only build the server"
            echo "  --agent-only     Only build the agent"
            echo "  --skip-web       Skip web frontend build"
            echo "  --universal      Build universal binary (x86_64 + arm64)"
            echo "  -h, --help       Show this help"
            exit 0
            ;;
        *)
            echo_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Detect architecture
ARCH=$(detect_arch)
echo_info "Detected Arch: $ARCH"

# Set target based on architecture
if [ "$ARCH" = "x86_64" ]; then
    TARGET="x86_64-apple-darwin"
elif [ "$ARCH" = "aarch64" ]; then
    TARGET="aarch64-apple-darwin"
else
    echo_error "Unsupported architecture: $ARCH"
    exit 1
fi

echo_info "Target: $TARGET"

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo_error "Rust not found. Please install from https://rustup.rs"
    exit 1
fi

echo_info "Rust version: $(rustc --version)"

# Check if Node.js is installed (required for web frontend)
if ! command -v node &> /dev/null; then
    echo_error "Node.js not found. Please install from https://nodejs.org or via Homebrew"
    echo_info "For Homebrew: brew install node"
    exit 1
else
    echo_info "Node.js version: $(node --version)"
    echo_info "npm version: $(npm --version)"
fi

# Check if target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
    echo_info "Installing target: $TARGET"
    rustup target add "$TARGET"
fi

# For universal builds, also install the other architecture
if [ "$UNIVERSAL" = true ]; then
    if [ "$ARCH" = "x86_64" ]; then
        OTHER_TARGET="aarch64-apple-darwin"
    else
        OTHER_TARGET="x86_64-apple-darwin"
    fi

    if ! rustup target list --installed | grep -q "$OTHER_TARGET"; then
        echo_info "Installing target for universal build: $OTHER_TARGET"
        rustup target add "$OTHER_TARGET"
    fi
fi

# Create output directory (platform-specific)
OUTPUT_DIR="./dist/macos"
mkdir -p "$OUTPUT_DIR"

# Build web frontend
if [ "$SKIP_WEB" = false ] && [ "$BUILD_AGENT" = false ] || [ "$BUILD_SERVER" = true ]; then
    echo ""
    echo "========================================"
    echo "  Building Web Frontend"
    echo "========================================"
    echo ""

    if [ -d "web" ] && [ -f "web/package.json" ]; then
        echo_info "Building web frontend..."
        cd web

        # Always ensure dependencies are installed
        echo_info "Installing npm dependencies..."
        npm install

        npm run build
        cd ..
        echo_info "Web frontend built successfully!"
    else
        echo_warn "web/package.json not found, skipping frontend build"
    fi
fi

echo ""
echo "========================================"
echo "  Building Rust Binaries"
echo "========================================"
echo ""

# Build server
if [ "$BUILD_SERVER" = true ]; then
    echo_info "Building claude-tunnel-server with $TARGET..."
    cargo build --package server --target "$TARGET" --release

    SERVER_BINARY="target/$TARGET/release/claude-tunnel-server"
    OUTPUT_SERVER="$OUTPUT_DIR/claude-tunnel-server"
    SERVER_HASH="$OUTPUT_DIR/claude-tunnel-server.hash.txt"

    if [ -f "$SERVER_BINARY" ]; then
        # Universal binary handling
        if [ "$UNIVERSAL" = true ]; then
            echo_info "Building for $OTHER_TARGET..."
            cargo build --package server --target "$OTHER_TARGET" --release

            OTHER_SERVER="target/$OTHER_TARGET/release/claude-tunnel-server"
            if [ -f "$OTHER_SERVER" ]; then
                echo_info "Creating universal binary..."
                lipo -create "$SERVER_BINARY" "$OTHER_SERVER" -output "$OUTPUT_SERVER"
            else
                echo_warn "Other architecture build failed, using single architecture"
                cp "$SERVER_BINARY" "$OUTPUT_SERVER"
            fi
        else
            cp "$SERVER_BINARY" "$OUTPUT_SERVER"
        fi

        chmod +x "$OUTPUT_SERVER"
        echo_info "Server build successful!"
        echo_info "Binary: $OUTPUT_SERVER"
        echo_info "Size: $(du -h $OUTPUT_SERVER | cut -f1)"

        # Generate hash file
        generate_hash "$OUTPUT_SERVER" "$SERVER_HASH"
        echo ""
    else
        echo_error "Server build failed: binary not found"
        exit 1
    fi
fi

# Build agent
if [ "$BUILD_AGENT" = true ]; then
    echo_info "Building claude-tunnel-agent with $TARGET..."
    cargo build --package agent --target "$TARGET" --release

    AGENT_BINARY="target/$TARGET/release/claude-tunnel-agent"
    OUTPUT_AGENT="$OUTPUT_DIR/claude-tunnel-agent"
    AGENT_HASH="$OUTPUT_DIR/claude-tunnel-agent.hash.txt"

    if [ -f "$AGENT_BINARY" ]; then
        # Universal binary handling
        if [ "$UNIVERSAL" = true ]; then
            echo_info "Building for $OTHER_TARGET..."
            cargo build --package agent --target "$OTHER_TARGET" --release

            OTHER_AGENT="target/$OTHER_TARGET/release/claude-tunnel-agent"
            if [ -f "$OTHER_AGENT" ]; then
                echo_info "Creating universal binary..."
                lipo -create "$AGENT_BINARY" "$OTHER_AGENT" -output "$OUTPUT_AGENT"
            else
                echo_warn "Other architecture build failed, using single architecture"
                cp "$AGENT_BINARY" "$OUTPUT_AGENT"
            fi
        else
            cp "$AGENT_BINARY" "$OUTPUT_AGENT"
        fi

        chmod +x "$OUTPUT_AGENT"
        echo_info "Agent build successful!"
        echo_info "Binary: $OUTPUT_AGENT"
        echo_info "Size: $(du -h $OUTPUT_AGENT | cut -f1)"

        # Generate hash file
        generate_hash "$OUTPUT_AGENT" "$AGENT_HASH"
        echo ""
    else
        echo_error "Agent build failed: binary not found"
        exit 1
    fi
fi

echo ""
echo "========================================"
echo "  Build Summary"
echo "========================================"
echo ""
echo_info "Output directory: $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR"
echo ""
echo "========================================"
echo "  Done!"
echo "========================================"
