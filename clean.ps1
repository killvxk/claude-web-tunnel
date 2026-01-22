# Claude Web Tunnel 清理脚本 (Windows)

param(
    [switch]$Deps,
    [switch]$Target,
    [switch]$Dist,
    [switch]$All,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }

if ($Help) {
    Write-Host @"
Claude Web Tunnel 清理脚本

用法: .\clean.ps1 [选项]

选项:
  -Deps       清理 node_modules
  -Target     清理 Rust target 目录
  -Dist       清理 dist 输出目录
  -All        清理所有 (Deps + Target + Dist)
  -Help       显示帮助

示例:
  .\clean.ps1 -Deps          # 只清理 node_modules
  .\clean.ps1 -All           # 清理所有
"@
    exit 0
}

if (-not ($Deps -or $Target -or $Dist -or $All)) {
    Write-Host "请指定清理选项。使用 -Help 查看帮助。"
    exit 0
}

if ($All) {
    $Deps = $true
    $Target = $true
    $Dist = $true
}

# Clean node_modules
if ($Deps) {
    if (Test-Path "web\node_modules") {
        Write-Info "删除 web\node_modules..."
        Remove-Item -Recurse -Force "web\node_modules"
        Write-Info "web\node_modules 已删除"
    } else {
        Write-Warn "web\node_modules 不存在"
    }
}

# Clean target
if ($Target) {
    if (Test-Path "target") {
        Write-Info "删除 target\..."
        Remove-Item -Recurse -Force "target"
        Write-Info "target\ 已删除"
    } else {
        Write-Warn "target\ 不存在"
    }
}

# Clean dist
if ($Dist) {
    if (Test-Path "dist") {
        Write-Info "删除 dist\..."
        Remove-Item -Recurse -Force "dist"
        Write-Info "dist\ 已删除"
    } else {
        Write-Warn "dist\ 不存在"
    }
}

Write-Info "清理完成!"
