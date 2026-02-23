//! Provider 模块
//!
//! 本模块实现了 LLM Provider 的正交分解架构，将每个 Provider 拆分为：
//! - **Protocol (协议层)**: 负责请求体的 JSON 格式和响应解析
//! - **Endpoint (端点层)**: 负责 URL 路由和认证注入
//!
//! # 核心协议
//!
//! 实际上只有 **3 种**核心协议格式：
//!
//! | 协议 | 特征 | 使用者 |
//! |------|------|--------|
//! | **Claude** | `system` 数组、`content` 数组、`tool_use`/`tool_result` | `ClaudeProvider` |
//! | **OpenAI** | `messages` 数组、`function` 工具、`image_url` | `OpenAIProvider`, `IFlowProvider` |
//! | **Gemini** | `contents` 数组、`role: "model"`、`parts`、`systemInstruction` | `GeminiProvider`, `VertexProvider` |
//!
//! # 协议变体
//!
//! 部分平台使用协议变体（方言壳）：
//!
//! | 变体 | 基于协议 | 说明 |
//! |------|----------|------|
//! | **CloudCode Protocol** | Gemini | 添加 `requestType: "agent"` 等外层字段 |
//! | **iFlow OpenAI** | OpenAI | 添加 `chat_template_kwargs.enable_thinking` 等字段 |
//!
//! # Provider 与协议对应关系
//!
//! ```text
//! Claude 协议
//! └── ClaudeProvider
//!
//! OpenAI 协议
//! ├── OpenAIProvider (官方)
//! └── IFlowProvider (iFlow 平台，添加 Thinking 字段)
//!
//! Gemini 协议
//! ├── GeminiProvider (官方 API Key)
//! ├── VertexProvider (GCP Service Account)
//! └── CloudCode Protocol (方言壳)
//!     ├── GeminiCliProvider
//!     └── AntigravityProvider
//! ```
//!
//! # 目录结构说明
//!
//! 目录按 Provider 划分（而非协议），原因：
//! 1. 用户视角直观：要使用某个 Provider → 找对应目录
//! 2. 认证/端点隔离：每个 Provider 有独立的认证配置
//! 3. 扩展方向：Provider 数量持续增长，协议相对稳定
//!
//! 协议复用通过 `Protocol` trait 和模块依赖实现（如 `gemini/protocol.rs`）。

pub mod traits;
pub mod gemini;
pub mod vertex;
pub mod claude;
pub mod openai;
pub mod iflow;
pub mod codex;
pub mod gemini_cli;
pub mod antigravity;

// 重导出
pub use traits::*;
