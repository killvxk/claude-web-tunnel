# GitHub Release 工作流设计

**日期**: 2026-02-25
**标签**: ci-cd, github-actions, release, multi-platform

## Summary

为 claude-web-tunnel 项目设计了 GitHub Actions 自动发布工作流。完成了项目结构全面分析（Rust workspace、Svelte 前端、多平台构建脚本），产出了详细的发布集成设计计划。计划已保存待用户确认后实施。

## Changes Made

- `docs/plan/github-release-plan.md` - 新建：GitHub Release 集成设计计划文档

## Decisions & Rationale

### 三 Job 流水线架构
- **决策**: 采用 build-web → build (矩阵) → release 三阶段流水线
- **理由**: 前端只需构建一次，通过 artifact 共享给所有平台；矩阵并行构建提高效率；Release 阶段统一收集产物

### Linux aarch64 使用 cross 交叉编译
- **决策**: 使用 cross-rs/cross 工具在 x86_64 runner 上交叉编译 aarch64-unknown-linux-musl
- **理由**: GitHub 没有 aarch64 Linux runner；cross 通过 Docker 容器提供完整的交叉编译环境，兼容 vendored OpenSSL

### Draft Release 模式
- **决策**: Release 创建为 Draft，需手动发布
- **理由**: 给维护者审核机会，确认产物正确后再公开发布

### 同时新增 CI 工作流
- **决策**: 除 release.yml 外也创建 ci.yml（cargo check + clippy + fmt）
- **理由**: PR/push 的基本质量门禁是发布流程的前提保障

## Technical Details

- **rust-embed 嵌入路径**: `crates/server/src/static_files.rs` 中 `#[folder = "../../web/dist"]`，server 编译前必须先构建前端
- **musl 静态链接**: Linux 使用 musl target 产出完全静态链接的二进制
- **Windows CRT 静态**: `.cargo/config.toml` 配置 `+crt-static` 用于 Windows 目标
- **发布产物命名**: `claude-tunnel-{server|agent}-{platform}-{arch}[.exe]`
- **校验和**: 合并 SHA256SUMS.txt 文件
- **触发方式**: tag push `v*` 或 workflow_dispatch 手动输入 tag

## Open Items / Follow-ups

- [ ] 用户确认设计方案
- [ ] 实施 `.github/workflows/release.yml`
- [ ] 实施 `.github/workflows/ci.yml`
- [ ] 验证工作流语法
- [ ] 首次发布测试（推送 tag 触发）

## Learnings

- rust-embed 在编译时嵌入文件，CI 中必须确保 web/dist/ 在 cargo build 前已存在
- cross 工具适合 CI 中的 musl 交叉编译场景，避免手动配置复杂的交叉编译工具链
- GitHub Actions 的 macOS runner 按架构区分：macos-13 (Intel x86_64)、macos-14 (M1 arm64)
