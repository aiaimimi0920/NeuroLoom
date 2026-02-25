//! # NeuroLoom LLM v2 Core Engine
//! 
//! 高度聚合的异步 LLM 客户端抽象库。提供极其稳固的按层解耦架构：
//! 
//! ## 🔌 核心架构亮点：多维认证形态脱钩 Matrix
//! 
//! 框架从 Preset 层针对同一家厂商实现了多业务线的完美脱钩。你通过 `LlmClient::from_preset("<ID>")` 直接指定认证法与模型路线，拒绝混淆：
//! 
//! - **Kimi**: 拆分为 `kimi` (通用 API), `kimi_coding` (专属编程 API), `kimi_oauth` (直接拉起浏览器通过 DeviceFlow 免密白嫖授权的 Web 通道)。
//! - **Qwen**: 拆分为 `qwen` (DashScope API), `qwen_coder` (最强代码模型通道), `qwen_oauth` (拉起鉴权登录 portal.qwen 体验中心的免密通道)。
//!
pub mod primitive;
pub mod provider;
pub mod site;
pub mod protocol;
pub mod auth;
pub mod model;
pub mod pipeline;
pub mod presets;
pub mod client;
pub mod concurrency;
pub mod metrics;

pub use client::LlmClient;
pub use primitive::request::PrimitiveRequest;
// [新增] 导出常用类型，便于外部使用
pub use site::{Site, SimpleSite, VertexSite, OpenAiSite, CloudCodeSite, AmpSite};
pub use site::context::{UrlContext, Action, AuthType};
pub use protocol::base::{OpenAiProtocol, GeminiProtocol, ClaudeProtocol};
pub use model::{ModelResolver, Capability, DefaultModelResolver};
pub use concurrency::{ConcurrencyConfig, ConcurrencyController, ConcurrencySnapshot, AdjustmentStrategy};
pub use metrics::{PipelineMetrics, MetricsStore, MetricsSummary};
