---
description: 优先使用中文进行回复
---

# NeuroLoom 开发规范

## 语言
优先使用中文进行回复和注释。

## 参考项目查阅规则

在添加新功能或修改现有功能之前，**必须先到 `references/` 目录中的参考项目查找相关代码**，看看是否有可复用的实现、设计模式或接口定义。

### 参考项目列表

| 目录 | 项目 | 说明 |
|------|------|------|
| `references/CLIProxyAPI/` | CLIProxyAPI (Go) | 核心代理，模型定义、OAuth、翻译器、执行器的权威参考 |
| `references/Antigravity-Manager/` | Antigravity Manager (Tauri+Vue) | Antigravity 桌面管理工具，账号管理、模型切换 |
| `references/cc-switch/` | cc-switch | Claude Code 多账号切换工具 |
| `references/claude-code-router/` | Claude Code Router | Claude Code 路由代理 |
| `references/new-api/` | new-api | API 网关/中转实现 |

### 查阅优先级

1. **CLIProxyAPI** — 涉及 OAuth、模型路由、请求翻译、CloudCode PA 协议时首先查阅
2. **Antigravity-Manager** — 涉及 Antigravity 账号管理、UI 交互时查阅
3. **cc-switch / claude-code-router** — 涉及 Claude Code 集成、多账号时查阅
4. **new-api** — 涉及 API 网关、OpenAI 兼容层时查阅

### 关键文件速查

- 模型定义: `references/CLIProxyAPI/internal/registry/model_definitions_static_data.go`
- Antigravity 执行器: `references/CLIProxyAPI/internal/runtime/executor/antigravity_executor.go`
- Claude→Antigravity 翻译: `references/CLIProxyAPI/internal/translator/antigravity/claude/`
- OAuth 模型别名: `references/CLIProxyAPI/sdk/cliproxy/auth/oauth_model_alias.go`
- 模型别名迁移: `references/CLIProxyAPI/internal/config/oauth_model_alias_migration.go`
