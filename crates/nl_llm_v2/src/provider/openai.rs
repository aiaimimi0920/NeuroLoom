use async_trait::async_trait;
use reqwest::Client;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

/// OpenAI 静态模型列表扩展
///
/// OpenAI 的 /models 端点返回的数据不可靠（实际可用模型与返回列表不一致），
/// 因此使用静态列表。模型数据来源：官方文档，截止 2026-02-25。
pub struct OpenAiExtension;

impl OpenAiExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenAiExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn openai_models() -> Vec<ModelInfo> {
    vec![
        // GPT-4o 系列（推荐）
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o — Flagship multimodal model, 128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini — Fast and affordable, 128K context".to_string(),
        },
        // GPT-4 Turbo
        ModelInfo {
            id: "gpt-4-turbo".to_string(),
            description: "GPT-4 Turbo — Previous generation, 128K context".to_string(),
        },
        // GPT-3.5
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            description: "GPT-3.5 Turbo — Fast and economical, 16K context".to_string(),
        },
        // 推理模型
        ModelInfo {
            id: "o1".to_string(),
            description: "o1 — Advanced reasoning model, 200K context".to_string(),
        },
        ModelInfo {
            id: "o1-mini".to_string(),
            description: "o1-mini — Fast reasoning model, 128K context".to_string(),
        },
        ModelInfo {
            id: "o1-pro".to_string(),
            description: "o1-pro — Premium reasoning model, 200K context".to_string(),
        },
        ModelInfo {
            id: "o3-mini".to_string(),
            description: "o3-mini — Latest reasoning model, 200K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for OpenAiExtension {
    fn id(&self) -> &str {
        "openai"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(openai_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // OpenAI 付费用户: 10,000 RPM
        // 按 10 秒平均响应时间计算，约 1000 并发
        // 使用保守值 100 作为默认
        ConcurrencyConfig::new(100)
    }
}

/// 返回 Arc 包装好的扩展实例（供 preset 使用）
pub fn extension() -> Arc<OpenAiExtension> {
    Arc::new(OpenAiExtension::new())
}
