# GitHub Release 集成设计计划

## 问题描述

项目当前没有任何 CI/CD 配置，所有构建依赖手动运行平台特定的脚本（build-linux.sh、build-macos.sh、build-windows.ps1）。需要设计 GitHub Actions 工作流，实现自动化的多平台构建与 GitHub Release 发布。

## 项目分析

### 产物

| 二进制 | Crate | 说明 |
|--------|-------|------|
| `claude-tunnel-server` | crates/server | 服务端，内嵌 Svelte 前端 (rust-embed) |
| `claude-tunnel-agent` | crates/agent | 客户端 Agent |

### 支持平台

| 平台 | Rust Target | 构建特点 |
|------|-------------|----------|
| Linux x86_64 | `x86_64-unknown-linux-musl` | musl 静态链接，需 musl-tools |
| Linux aarch64 | `aarch64-unknown-linux-musl` | musl 静态链接，需交叉编译 |
| macOS arm64 | `aarch64-apple-darwin` | 原生构建 |
| Windows x64 | `x86_64-pc-windows-msvc` | CRT 静态链接 |

### 关键依赖关系

- **前端构建**：Server 通过 `rust-embed` 在编译时内嵌 `web/dist/`，必须先构建前端
- **OpenSSL vendored**：Agent 在 musl 目标使用 `openssl = { features = ["vendored"] }`，从源码编译
- **mimalloc**：musl 目标使用 mimalloc 替代默认分配器

## 解决方案

### 工作流设计

创建 `.github/workflows/release.yml`，包含 3 个 Job：

```
push tag v* ──→ build-web ──→ build (4 平台矩阵) ──→ release
                  │                    │                    │
             Svelte 前端          下载前端产物          下载所有二进制
             npm ci + build       Rust 编译             创建 Draft Release
             上传 artifact        上传 artifact         附加所有文件
```

### 触发方式

1. **Tag 推送**：推送 `v*` 格式标签自动触发（主要方式）
2. **手动触发**：`workflow_dispatch` 支持输入 tag 名手动发布

### 构建矩阵

| 名称 | Runner | Target | 交叉编译 |
|------|--------|--------|----------|
| linux-x86_64 | ubuntu-22.04 | x86_64-unknown-linux-musl | 否 |
| linux-aarch64 | ubuntu-22.04 | aarch64-unknown-linux-musl | 是 (cross) |
| macos-aarch64 | macos-14 | aarch64-apple-darwin | 否 |
| windows-x64 | windows-latest | x86_64-pc-windows-msvc | 否 |

### 发布产物

| 平台 | 产物 | 内容 |
|------|------|------|
| Linux x86_64 | `claude-web-tunnel-linux-x86_64.tar.gz` | Server + Agent + deploy/ 安装脚本 |
| Linux aarch64 | `claude-web-tunnel-linux-aarch64.tar.gz` | Server + Agent + deploy/ 安装脚本 |
| macOS arm64 | `claude-tunnel-agent-macos-aarch64` | Agent 二进制 |
| Windows x64 | `claude-tunnel-agent-windows-x64.exe` | Agent 二进制 |
| 通用 | `SHA256SUMS.txt` | 所有文件校验和 |

### Release 特性

- **Draft 模式**：Release 创建为草稿，需手动审核后发布
- **预发布检测**：Tag 含 `-`（如 `v0.1.0-beta.1`）自动标记为 prerelease
- **自动生成 Release Notes**：基于 commit 历史自动生成变更说明
- **安装说明**：Release body 包含平台对照表和快速启动指引
- **校验和**：合并的 SHA256SUMS.txt 文件

## 修改清单

| # | 文件 | 操作 | 说明 |
|---|------|------|------|
| 1 | `.github/workflows/release.yml` | 新建 | Release 工作流 |
| 2 | `.github/workflows/ci.yml` | 新建 | CI 工作流（PR/push 触发，cargo check + clippy + fmt） |

## 状态检查列表

- [x] 创建 `.github/workflows/release.yml`
- [x] 创建 `.github/workflows/ci.yml`
- [x] 验证工作流语法正确性 (yaml-lint 通过)
