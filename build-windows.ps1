# Claude Web Tunnel Windows Build Script
# Requires: Rust toolchain with MSVC target

param(
    [switch]$ServerOnly,
    [switch]$AgentOnly,
    [switch]$SkipWeb,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }

if ($Help) {
    Write-Host @"
Claude Web Tunnel Windows Build Script

Usage: .\build-windows.ps1 [options]

Options:
  -ServerOnly    Only build the server
  -AgentOnly     Only build the agent
  -SkipWeb       Skip web frontend build
  -Help          Show this help

Examples:
  .\build-windows.ps1                   # Build both server and agent
  .\build-windows.ps1 -AgentOnly        # Build only agent
  .\build-windows.ps1 -SkipWeb          # Build without web frontend
"@
    exit 0
}

Write-Host "========================================"
Write-Host "  Claude Web Tunnel Windows Build"
Write-Host "========================================"
Write-Host ""

# Check Rust installation
Write-Info "Checking Rust installation..."
try {
    $rustVersion = rustc --version
    Write-Info "Rust version: $rustVersion"
} catch {
    Write-Error "Rust not found. Please install from https://rustup.rs"
    exit 1
}

# Check Node.js installation (required for web frontend)
Write-Info "Checking Node.js installation..."
try {
    $nodeVersion = node --version
    $npmVersion = npm --version
    Write-Info "Node.js version: $nodeVersion"
    Write-Info "npm version: $npmVersion"
} catch {
    Write-Error "Node.js not found. Please install from https://nodejs.org"
    exit 1
}

# Get target
$arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
if ($arch -eq "X64") {
    $target = "x86_64-pc-windows-msvc"
} elseif ($arch -eq "Arm64") {
    $target = "aarch64-pc-windows-msvc"
} else {
    Write-Error "Unsupported architecture: $arch"
    exit 1
}

Write-Info "Target: $target"

# Check if target is installed
$installedTargets = rustup target list --installed
if ($installedTargets -notcontains $target) {
    Write-Info "Installing target: $target"
    rustup target add $target
}

# Create output directory (platform-specific)
$outputDir = ".\dist\windows"
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

# Build web frontend
if (-not $SkipWeb -and -not $AgentOnly) {
    Write-Host ""
    Write-Host "========================================"
    Write-Host "  Building Web Frontend"
    Write-Host "========================================"
    Write-Host ""

    if (Test-Path "web\package.json") {
        Push-Location web

        # Always ensure dependencies are installed
        Write-Info "Installing npm dependencies..."
        npm install

        Write-Info "Building web frontend..."
        npm run build

        Pop-Location
        Write-Info "Web frontend built successfully!"
    } else {
        Write-Warn "web\package.json not found, skipping frontend build"
    }
}

Write-Host ""
Write-Host "========================================"
Write-Host "  Building Rust Binaries"
Write-Host "========================================"
Write-Host ""

# Build server
if (-not $AgentOnly) {
    Write-Info "Building claude-tunnel-server..."
    cargo build --package server --target $target --release

    $serverBinary = "target\$target\release\claude-tunnel-server.exe"
    $outputServer = "$outputDir\claude-tunnel-server.exe"

    if (Test-Path $serverBinary) {
        Copy-Item $serverBinary $outputServer -Force
        $size = (Get-Item $outputServer).Length / 1MB
        Write-Info "Server build successful!"
        Write-Info "Binary: $outputServer"
        Write-Info ("Size: {0:N2} MB" -f $size)

        # Generate hash
        $hash = Get-FileHash $outputServer -Algorithm SHA256
        $hashFile = "$outputDir\claude-tunnel-server.hash.txt"
        @"
# Claude Web Tunnel Server Build Hash
# Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss UTC")

File: claude-tunnel-server.exe
Size: $("{0:N2} MB ({1} bytes)" -f $size, (Get-Item $outputServer).Length)

SHA256: $($hash.Hash)
"@ | Out-File $hashFile -Encoding UTF8
        Write-Info "Hash file: $hashFile"
        Write-Host ""
    } else {
        Write-Error "Server build failed: binary not found"
        exit 1
    }
}

# Build agent
if (-not $ServerOnly) {
    Write-Info "Building claude-tunnel-agent..."
    cargo build --package agent --target $target --release

    $agentBinary = "target\$target\release\claude-tunnel-agent.exe"
    $outputAgent = "$outputDir\claude-tunnel-agent.exe"

    if (Test-Path $agentBinary) {
        Copy-Item $agentBinary $outputAgent -Force
        $size = (Get-Item $outputAgent).Length / 1MB
        Write-Info "Agent build successful!"
        Write-Info "Binary: $outputAgent"
        Write-Info ("Size: {0:N2} MB" -f $size)

        # Generate hash
        $hash = Get-FileHash $outputAgent -Algorithm SHA256
        $hashFile = "$outputDir\claude-tunnel-agent.hash.txt"
        @"
# Claude Web Tunnel Agent Build Hash
# Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss UTC")

File: claude-tunnel-agent.exe
Size: $("{0:N2} MB ({1} bytes)" -f $size, (Get-Item $outputAgent).Length)

SHA256: $($hash.Hash)
"@ | Out-File $hashFile -Encoding UTF8
        Write-Info "Hash file: $hashFile"
        Write-Host ""
    } else {
        Write-Error "Agent build failed: binary not found"
        exit 1
    }
}

Write-Host ""
Write-Host "========================================"
Write-Host "  Build Summary"
Write-Host "========================================"
Write-Host ""
Write-Info "Output directory: $outputDir"
Get-ChildItem $outputDir | Format-Table Name, Length, LastWriteTime
Write-Host "========================================"
Write-Host "  Done!"
Write-Host "========================================"
