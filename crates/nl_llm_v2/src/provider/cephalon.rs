use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// Cephalon 默认 API 基础 URL
const DEFAULT_BASE_URL: &str = "https://cephalon.cloud/user-center/v1/model";

/// Cephalon 平台扩展
///
/// Cephalon 是一个 AI 模型聚合平台，提供多种 LLM 模型服务。
///
/// ## 核心特性
///
/// - **协议**: OpenAI 兼容
/// - **认证**: Bearer Token
/// - **静态模型列表**: 平台提供多种主流模型
///
/// ## 模型说明
///
/// Cephalon 提供多种主流 LLM 模型，包括：
/// - OpenAI 系列 (gpt-4o, gpt-4-turbo, gpt-3.5-turbo)
/// - Claude 系列 (claude-3-opus, claude-3-sonnet, claude-3-haiku)
/// - 其他开源模型
///
/// ## 并发策略
///
/// 采用保守的并发配置，避免触发平台限流。
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("cephalon")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
pub struct CephalonExtension {
    base_url: String,
}

impl CephalonExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL（用于代理场景）
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Default for CephalonExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn cephalon_models() -> Vec<ModelInfo> {
    vec![
        // OpenAI 系列
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o — Flagship multimodal model".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini — Fast and affordable".to_string(),
        },
        ModelInfo {
            id: "gpt-4-turbo".to_string(),
            description: "GPT-4 Turbo — Previous generation, 128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            description: "GPT-3.5 Turbo — Fast and economical".to_string(),
        },
        // Claude 系列
        ModelInfo {
            id: "claude-3-opus-20240229".to_string(),
            description: "Claude 3 Opus — Most capable Claude model".to_string(),
        },
        ModelInfo {
            id: "claude-3-sonnet-20240229".to_string(),
            description: "Claude 3 Sonnet — Balanced performance".to_string(),
        },
        ModelInfo {
            id: "claude-3-haiku-20240307".to_string(),
            description: "Claude 3 Haiku — Fast and efficient".to_string(),
        },
        // DeepSeek 系列
        ModelInfo {
            id: "deepseek-chat".to_string(),
            description: "DeepSeek Chat — 通用对话模型".to_string(),
        },
        ModelInfo {
            id: "deepseek-reasoner".to_string(),
            description: "DeepSeek Reasoner — 深度推理模型".to_string(),
        },
        // 其他模型
        ModelInfo {
            id: "gemini-1.5-pro".to_string(),
            description: "Gemini 1.5 Pro — Google's multimodal model".to_string(),
        },
        ModelInfo {
            id: "gemini-1.5-flash".to_string(),
            description: "Gemini 1.5 Flash — Fast multimodal model".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for CephalonExtension {
    fn id(&self) -> &str {
        "cephalon"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(cephalon_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 使用保守的并发配置
        ConcurrencyConfig::new(20)
    }
}

pub fn extension() -> Arc<CephalonExtension> {
    Arc::new(CephalonExtension::new())
}
