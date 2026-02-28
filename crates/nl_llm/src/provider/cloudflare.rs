//! Cloudflare Workers AI 扩展
//!
//! Cloudflare Workers AI 提供在全球边缘网络上运行的 AI 模型。
//!
//! ## API 端点
//!
//! - **OpenAI 兼容**: `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1/chat/completions`
//! - **原生 API**: `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/run/{model}`
//! - **认证**: `Authorization: Bearer <API_Token>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 |
//! |---------|------|
//! | `@cf/meta/llama-3.1-8b-instruct` | Llama 3.1 8B — 免费，快速 |
//! | `@cf/meta/llama-3.1-70b-instruct` | Llama 3.1 70B — 强力 |
//! | `@cf/meta/llama-3.3-70b-instruct-fp8-fast` | Llama 3.3 70B FP8 — 最新 |
//! | `@cf/mistral/mistral-7b-instruct-v0.2-lora` | Mistral 7B |
//! | `@cf/qwen/qwen1.5-14b-chat-awq` | Qwen 1.5 14B |
//! | `@hf/google/gemma-7b-it` | Gemma 7B |
//! | `@cf/deepseek-ai/deepseek-r1-distill-qwen-32b` | DeepSeek R1 |
//!
//! ## 免费额度
//!
//! - 每日 10,000 神经元（Neurons）免费额度
//! - 超出后按量付费
//!
//! ## 并发策略
//!
//! - 默认: 50 并发
//! - 初始: 10

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

pub struct CloudflareExtension {
    account_id: String,
}

impl CloudflareExtension {
    pub fn new() -> Self {
        Self {
            account_id: String::new(),
        }
    }

    pub fn with_account_id(mut self, id: impl Into<String>) -> Self {
        self.account_id = id.into();
        self
    }

    /// 构建 OpenAI 兼容模式的 base URL
    pub fn base_url(&self) -> String {
        format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/v1",
            self.account_id
        )
    }
}

impl Default for CloudflareExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn cloudflare_models() -> Vec<ModelInfo> {
    vec![
        // === Meta Llama ===
        ModelInfo {
            id: "@cf/meta/llama-3.3-70b-instruct-fp8-fast".to_string(),
            description: "Llama 3.3 70B FP8 — 最新最强".to_string(),
        },
        ModelInfo {
            id: "@cf/meta/llama-3.1-70b-instruct".to_string(),
            description: "Llama 3.1 70B — 强力模型".to_string(),
        },
        ModelInfo {
            id: "@cf/meta/llama-3.1-8b-instruct".to_string(),
            description: "Llama 3.1 8B — 免费快速".to_string(),
        },
        // === DeepSeek ===
        ModelInfo {
            id: "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b".to_string(),
            description: "DeepSeek R1 Distill Qwen 32B — 推理模型".to_string(),
        },
        // === Mistral ===
        ModelInfo {
            id: "@cf/mistral/mistral-7b-instruct-v0.2-lora".to_string(),
            description: "Mistral 7B — 轻量对话".to_string(),
        },
        // === Qwen ===
        ModelInfo {
            id: "@cf/qwen/qwen1.5-14b-chat-awq".to_string(),
            description: "Qwen 1.5 14B — 中文优化".to_string(),
        },
        // === Google ===
        ModelInfo {
            id: "@hf/google/gemma-7b-it".to_string(),
            description: "Gemma 7B — Google 开源".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for CloudflareExtension {
    fn id(&self) -> &str {
        "cloudflare"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(cloudflare_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None) // 通过 Cloudflare Dashboard 查看用量
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 50,
            initial_limit: 10,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<CloudflareExtension> {
    Arc::new(CloudflareExtension::new())
}
