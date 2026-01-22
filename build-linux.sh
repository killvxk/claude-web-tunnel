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

# Function to detect the operating system
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$NAME
    elif type lsb_release >/dev/null 2>&1; then
        OS=$(lsb_release -si)
    elif [ -f /etc/lsb-release ]; then
        . /etc/lsb-release
        OS=$DISTRIB_ID
    elif [ -f /etc/debian_version ]; then
        OS="Debian"
    elif [ -f /etc/redhat-release ]; then
        OS="Red Hat"
    elif [ -f /etc/rocky-release ]; then
        OS="Rocky Linux"
    elif [ -f /etc/alpine-release ]; then
        OS="Alpine"
    else
        OS=$(uname -s)
    fi
    echo "$OS"
}

# Function to detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case $ARCH in
        x86_64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            echo "$ARCH"
            ;;
    esac
}

# Function to install dependencies on Debian-based systems
install_deps_debian() {
    echo_info "Installing dependencies for Debian/Ubuntu..."
    sudo apt update
    sudo apt install -y curl build-essential musl-tools musl-dev pkg-config libssl-dev
}

# Function to install dependencies on Red Hat-based systems
install_deps_redhat() {
    echo_info "Installing dependencies for Red Hat/CentOS/Fedora..."
    sudo yum update -y || sudo dnf update -y
    sudo yum groupinstall -y "Development Tools" || sudo dnf groupinstall -y "Development Tools"
    sudo yum install -y curl musl-gcc musl-libc-static openssl-devel || sudo dnf install -y curl musl-gcc musl-libc-static openssl-devel || true
}

# Function to install dependencies on Arch-based systems
install_deps_arch() {
    echo_info "Installing dependencies for Arch Linux..."
    sudo pacman -Syu --noconfirm
    sudo pacman -S --noconfirm base-devel git musl openssl
}

# Function to install dependencies on Alpine
install_deps_alpine() {
    echo_info "Installing dependencies for Alpine..."
    apk update
    apk add curl build-base musl-dev openssl-dev
}

# Function to install dependencies on openSUSE
install_deps_suse() {
    echo_info "Installing dependencies for openSUSE..."
    sudo zypper refresh
    sudo zypper install -y curl gcc make musl libopenssl-devel
}

# Function to install Rust
install_rust() {
    echo_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
}

# Function to generate hash file
generate_hash() {
    local binary=$1
    local hash_file=$2

    echo_info "Generating hash file: $hash_file"

    # Get current date
    local build_date=$(date -u '+%Y-%m-%d %H:%M:%S UTC')

    # Calculate hashes
    local sha256=$(sha256sum "$binary" | awk '{print $1}')
    local md5=$(md5sum "$binary" | awk '{print $1}')
    local size=$(du -h "$binary" | cut -f1)
    local size_bytes=$(stat -c%s "$binary" 2>/dev/null || stat -f%z "$binary" 2>/dev/null)

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
echo "  Claude Web Tunnel Build Script"
echo "========================================"
echo ""

# Parse arguments
BUILD_SERVER=true
BUILD_AGENT=true
SKIP_DEPS=false

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
        --skip-deps)
            SKIP_DEPS=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --server-only    Only build the server"
            echo "  --agent-only     Only build the agent"
            echo "  --skip-deps      Skip dependency installation"
            echo "  -h, --help       Show this help"
            exit 0
            ;;
        *)
            echo_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Detect OS and architecture
OS=$(detect_os)
ARCH=$(detect_arch)
echo_info "Detected OS: $OS"
echo_info "Detected Arch: $ARCH"

# Set musl target based on architecture
if [ "$ARCH" = "x86_64" ]; then
    MUSL_TARGET="x86_64-unknown-linux-musl"
elif [ "$ARCH" = "aarch64" ]; then
    MUSL_TARGET="aarch64-unknown-linux-musl"
else
    echo_error "Unsupported architecture: $ARCH"
    exit 1
fi

echo_info "MUSL target: $MUSL_TARGET"
echo ""

# Install system dependencies
if [ "$SKIP_DEPS" = false ]; then
    case $OS in
        *Debian*|*Ubuntu*|*Mint*)
            install_deps_debian
            ;;
        *Red\ Hat*|*CentOS*|*Fedora*|*Rocky*|*AlmaLinux*|*Oracle*)
            install_deps_redhat
            ;;
        *Arch*|*Manjaro*)
            install_deps_arch
            ;;
        *Alpine*)
            install_deps_alpine
            ;;
        *SUSE*|*openSUSE*)
            install_deps_suse
            ;;
        *)
            echo_warn "Unknown OS: $OS, skipping dependency installation"
            echo_warn "Please ensure build-essential and musl-tools are installed"
            ;;
    esac
fi

# Function to install Node.js via nvm
install_nodejs() {
    echo_info "Installing Node.js via nvm..."

    # Download and install nvm
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash

    # Load nvm without restarting shell
    export NVM_DIR="$HOME/.nvm"
    [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

    # Install latest LTS Node.js
    nvm install --lts

    echo_info "Node.js installed successfully: $(node --version)"
}

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo_info "Rust not found, installing..."
    install_rust
else
    echo_info "Rust is already installed: $(rustc --version)"
fi

# Check if Node.js is installed (required for web frontend)
# First try to load nvm if it exists
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

if ! command -v node &> /dev/null; then
    echo_info "Node.js not found, installing via nvm..."
    install_nodejs
else
    echo_info "Node.js is already installed: $(node --version)"
    echo_info "npm version: $(npm --version)"
fi

# Source cargo env
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Check if stable toolchain is installed
if ! rustup show | grep -q "stable"; then
    echo_info "Installing stable toolchain..."
    rustup install stable
    rustup default stable
fi

# Check if musl target is installed
if rustup target list --installed | grep -q "$MUSL_TARGET"; then
    echo_info "MUSL target is already installed"
else
    echo_info "Installing MUSL target: $MUSL_TARGET"
    rustup target add "$MUSL_TARGET"
fi

# Update Rust
echo_info "Updating Rust..."
rustup update

# Create output directory (platform-specific)
OUTPUT_DIR="./dist/linux"
mkdir -p "$OUTPUT_DIR"

echo ""
echo "========================================"
echo "  Building Web Frontend"
echo "========================================"
echo ""

# Build web frontend first (required for embedding in server)
if [ -d "web" ]; then
    if [ -f "web/package.json" ]; then
        echo_info "Building web frontend..."
        cd web

        # Always ensure dependencies are installed
        echo_info "Installing npm dependencies..."
        npm install

        # Build the frontend
        npm run build

        cd ..
        echo_info "Web frontend built successfully!"
    else
        echo_warn "web/package.json not found, skipping frontend build"
    fi
else
    echo_warn "web directory not found, skipping frontend build"
fi

echo ""
echo "========================================"
echo "  Building Claude Web Tunnel"
echo "========================================"
echo ""

# Build server
if [ "$BUILD_SERVER" = true ]; then
    echo_info "Building claude-tunnel-server with $MUSL_TARGET..."
    cargo build --package server --target "$MUSL_TARGET" --release

    SERVER_BINARY="target/$MUSL_TARGET/release/claude-tunnel-server"
    OUTPUT_SERVER="$OUTPUT_DIR/claude-tunnel-server"
    SERVER_HASH="$OUTPUT_DIR/claude-tunnel-server.hash.txt"

    if [ -f "$SERVER_BINARY" ]; then
        cp "$SERVER_BINARY" "$OUTPUT_SERVER"
        chmod +x "$OUTPUT_SERVER"
        echo_info "Server build successful!"
        echo_info "Binary: $OUTPUT_SERVER"
        echo_info "Size: $(du -h $OUTPUT_SERVER | cut -f1)"

        # Verify it's statically linked
        if command -v file &> /dev/null; then
            echo_info "Binary info:"
            file "$OUTPUT_SERVER"
        fi

        # Generate hash file
        generate_hash "$OUTPUT_SERVER" "$SERVER_HASH"
        echo ""
    else
        echo_error "Server build failed: binary not found at $SERVER_BINARY"
        exit 1
    fi
fi

# Build agent
if [ "$BUILD_AGENT" = true ]; then
    echo_info "Building claude-tunnel-agent with $MUSL_TARGET..."
    cargo build --package agent --target "$MUSL_TARGET" --release

    AGENT_BINARY="target/$MUSL_TARGET/release/claude-tunnel-agent"
    OUTPUT_AGENT="$OUTPUT_DIR/claude-tunnel-agent"
    AGENT_HASH="$OUTPUT_DIR/claude-tunnel-agent.hash.txt"

    if [ -f "$AGENT_BINARY" ]; then
        cp "$AGENT_BINARY" "$OUTPUT_AGENT"
        chmod +x "$OUTPUT_AGENT"
        echo_info "Agent build successful!"
        echo_info "Binary: $OUTPUT_AGENT"
        echo_info "Size: $(du -h $OUTPUT_AGENT | cut -f1)"

        # Verify it's statically linked
        if command -v file &> /dev/null; then
            echo_info "Binary info:"
            file "$OUTPUT_AGENT"
        fi

        # Generate hash file
        generate_hash "$OUTPUT_AGENT" "$AGENT_HASH"
        echo ""
    else
        echo_error "Agent build failed: binary not found at $AGENT_BINARY"
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
