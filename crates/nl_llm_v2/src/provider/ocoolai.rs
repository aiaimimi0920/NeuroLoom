use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// ocoolAI 平台扩展
///
/// 提供 ocoolAI 平台特定的功能实现。
pub struct OcoolAiExtension {
    base_url: String,
}

impl OcoolAiExtension {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.ocoolai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Default for OcoolAiExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// ocoolAI 热门模型列表
///
/// ocoolAI 平台的 /models 端点返回的数据可能不完整，
/// 因此使用静态列表维护主流模型。数据来源：平台官网 2026-02。
fn ocoolai_models() -> Vec<ModelInfo> {
    vec![
        // GPT-4o 系列
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o — Flagship multimodal model, 128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini — Fast and affordable, 128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4-turbo".to_string(),
            description: "GPT-4 Turbo — Previous generation, 128K context".to_string(),
        },
        // GPT-3.5
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            description: "GPT-3.5 Turbo — Fast and economical, 16K context".to_string(),
        },
        // Claude 系列
        ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            description: "Claude 3.5 Sonnet — Latest Claude model, 200K context".to_string(),
        },
        ModelInfo {
            id: "claude-3-opus-20240229".to_string(),
            description: "Claude 3 Opus — Most capable Claude model, 200K context".to_string(),
        },
        ModelInfo {
            id: "claude-3-haiku-20240307".to_string(),
            description: "Claude 3 Haiku — Fast and efficient, 200K context".to_string(),
        },
        // Gemini 系列
        ModelInfo {
            id: "gemini-1.5-pro".to_string(),
            description: "Gemini 1.5 Pro — Advanced reasoning, 1M context".to_string(),
        },
        ModelInfo {
            id: "gemini-1.5-flash".to_string(),
            description: "Gemini 1.5 Flash — Fast and efficient, 1M context".to_string(),
        },
        // DeepSeek 系列
        ModelInfo {
            id: "deepseek-chat".to_string(),
            description: "DeepSeek V3 — General purpose chat, 64K context".to_string(),
        },
        ModelInfo {
            id: "deepseek-reasoner".to_string(),
            description: "DeepSeek R1 — Deep reasoning model, 64K context".to_string(),
        },
        // Llama 系列
        ModelInfo {
            id: "llama-3.1-405b".to_string(),
            description: "Llama 3.1 405B — Largest Llama model, 128K context".to_string(),
        },
        // Qwen 系列
        ModelInfo {
            id: "qwen-max".to_string(),
            description: "Qwen Max — Advanced reasoning, 32K context".to_string(),
        },
        // GLM 系列
        ModelInfo {
            id: "glm-4".to_string(),
            description: "GLM-4 — Zhipu AI flagship model, 128K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for OcoolAiExtension {
    fn id(&self) -> &str {
        "ocoolai"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // ocoolAI 的 /models 端点可能不完整，使用静态列表
        Ok(ocoolai_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // ocoolAI 作为中转平台，使用保守的并发配置
        // 具体限制取决于用户账户等级
        ConcurrencyConfig::new(50)
    }
}

/// 返回 Arc 包装好的扩展实例（供 preset 使用）
pub fn extension() -> Arc<OcoolAiExtension> {
    Arc::new(OcoolAiExtension::new())
}
