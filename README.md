# NeuroLoom (神织机)

SOTA 级智能体操作系统 - 将大模型的神经编织进物理操作系统的底层逻辑。

## 发音

`/ˈnjʊəroʊ luːm/` - "New-row-loom"

- **Neuro**: 神经，代表大模型飞速运转的并发神经元
- **Loom**: 织机，代表底层坚如磐石的事件溯源数据库

## 架构

```
NeuroLoom/
├── apps/
│   ├── daemon/      # [核心引擎] Headless 后台守护进程
│   ├── desktop/     # [空间画布] Tauri 流式前端
│   └── cli/         # [极客终端] 命令行交互接口
│
└── crates/
    ├── nl_core/      # 核心原语 (事件溯源, UUID, 错误处理)
    ├── nl_durable/   # 持久化底座 (SQLite, Actor Mesh)
    ├── nl_llm/       # 算力与网关 (Prompt AST, 令牌桶, 黑魔法代理)
    ├── nl_memory/    # 记忆底座 (HAMT, GraphRAG)
    ├── nl_cognitive/ # 认知与法庭 (SOP, MCTS, MoA)
    ├── nl_sandbox/   # 物理执行网 (God Mode, Micro-VM)
    ├── nl_vision/    # 视觉流 (语义帧差分)
    └── nl_hap/       # 星际联邦协议 (HAP)
```

## 核心设计哲学

1. **绝对解耦 (Model-Agnostic)**: Prompt AST 与防腐层隔离模型方言
2. **基底与不死底座**: 事件溯源 + Actor Mesh
3. **空间交互**: CRDTs 空间流式画布
4. **网络与万维联邦**: HAP 协议
5. **权限与物理真理**: 执行即法官
6. **算力与进化**: 无感重试 + MoA 议会

## nl_llm 黑魔法代理

整合四个开源项目的代理形态：

| 项目 | 代理形态 | 认证方式 |
|------|----------|----------|
| [CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI) | `Api` + `Cli` | OAuth、API Key、多账户轮询 |
| [new-api](https://github.com/Calcium-Ion/new-api) | `Api` + `Auth` | API Key、OAuth |
| [cc-switch](https://github.com/farion1231/cc-switch) | `Api` + `WebSocket` | 本地配置 |
| [claude-code-router](https://github.com/musistudio/claude-code-router) | `Api` + `WebSocket` | x-api-key |

## 快速开始

```bash
# 构建项目
cargo build

# 运行测试
cargo test

# 运行守护进程
cargo run --bin neuroloom-daemon

# 运行 CLI
cargo run --bin nl
```

## 开发里程碑

- [x] **Phase 1**: 引擎点火 (nl_core, nl_llm 基础骨架)
- [ ] **Phase 2**: 认知与沙箱 (法庭闭环)
- [ ] **Phase 3**: 空间画布 (Tauri 前端)

## 文档

详细架构文档: [docs/架构.md](docs/架构.md)

## 许可证

MIT OR Apache-2.0
