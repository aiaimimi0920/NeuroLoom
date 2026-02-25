//! Cloudflare Workers AI 预设
//!
//! Cloudflare Workers AI 提供在全球边缘网络上运行的 AI 模型，具有低延迟和免费额度。
//!
//! ## API 端点
//!
//! - **OpenAI 兼容**: `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1/chat/completions`
//! - **原生 API**: `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/run/{model}`
//! - **认证**: `Authorization: Bearer <API_Token>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文长度 | 能力 |
//! |---------|------|-----------|------|
//! | `@cf/meta/llama-3.3-70b-instruct-fp8-fast` | Llama 3.3 70B FP8 — 最新最强（默认） | 131K | Chat, Tools, Streaming |
//! | `@cf/meta/llama-3.1-70b-instruct` | Llama 3.1 70B — 强力模型 | 131K | Chat, Tools, Streaming |
//! | `@cf/meta/llama-3.1-8b-instruct` | Llama 3.1 8B — 免费快速 | 131K | Chat, Tools, Streaming |
//! | `@cf/deepseek-ai/deepseek-r1-distill-qwen-32b` | DeepSeek R1 — 推理模型 | 65K | Chat, Thinking, Streaming |
//! | `@cf/mistral/mistral-7b-instruct-v0.2-lora` | Mistral 7B — 轻量对话 | 32K | Chat, Streaming |
//! | `@cf/qwen/qwen1.5-14b-chat-awq` | Qwen 1.5 14B — 中文优化 | 32K | Chat, Streaming |
//! | `@hf/google/gemma-7b-it` | Gemma 7B — Google 开源 | 8K | Chat, Streaming |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `cloudflare` / `llama` / `llama-3.3` | @cf/meta/llama-3.3-70b-instruct-fp8-fast |
//! | `llama-70b` | @cf/meta/llama-3.1-70b-instruct |
//! | `llama-8b` | @cf/meta/llama-3.1-8b-instruct |
//! | `deepseek` / `r1` | @cf/deepseek-ai/deepseek-r1-distill-qwen-32b |
//! | `mistral` | @cf/mistral/mistral-7b-instruct-v0.2-lora |
//! | `qwen` | @cf/qwen/qwen1.5-14b-chat-awq |
//! | `gemma` | @hf/google/gemma-7b-it |
//!
//! ## 使用方式
//!
//! Cloudflare Workers AI 需要 **Account ID** 和 **API Token**。
//!
//! ```rust,no_run
//! use nl_llm_v2::LlmClient;
//!
//! let client = LlmClient::from_preset("cloudflare")?
//!     .with_api_key("YOUR-API-TOKEN")
//!     .build();
//! ```
//!
//! ## 免费额度
//!
//! - 每日 10,000 神经元（Neurons）免费
//! - Llama 3.1 8B 几乎免费可用
//! - 超出后按量付费
//!
//! ## 并发策略
//!
//! - 官方最大: 50 并发
//! - 初始: 10

use crate::client::ClientBuilder;
use crate::model::cloudflare::CloudflareModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::cloudflare::CloudflareExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// 默认 Account ID — 需要用户在 .env.local 或环境变量中覆盖
const DEFAULT_ACCOUNT_ID: &str = "bea2e3be0577ee4f7b3ffae4df6f53bb";

fn cloudflare_base_url(account_id: &str) -> String {
    format!("https://api.cloudflare.com/client/v4/accounts/{}/ai/v1", account_id)
}

/// 创建 Cloudflare Workers AI 客户端构建器
pub fn builder() -> ClientBuilder {
    let base_url = cloudflare_base_url(DEFAULT_ACCOUNT_ID);
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(CloudflareModelResolver::new())
        .with_extension(Arc::new(
            CloudflareExtension::new().with_account_id(DEFAULT_ACCOUNT_ID)
        ))
        .default_model("@cf/meta/llama-3.3-70b-instruct-fp8-fast")
}
