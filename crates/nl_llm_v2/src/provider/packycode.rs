use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// PackyCode 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.packycode.com/v1";

/// PackyCode 聚合平台扩展
///
/// PackyCode 是国内 AI 模型 API 中转服务平台，
/// 提供统一 API 端点接入 OpenAI、Claude、Gemini 等多种模型。
/// 兼容 OpenAI 协议。
///
/// ## 认证方式
///
/// 标准 `Authorization: Bearer <key>` 格式，密钥 `sk-` 前缀。
///
/// ## 支持的模型
///
/// | 模型 ID | 说明 |
/// |---------|------|
/// | `gpt-4o` | GPT-4o |
/// | `gpt-4o-mini` | GPT-4o Mini |
/// | `gpt-4.1` | GPT-4.1 |
/// | `gpt-4.1-mini` | GPT-4.1 Mini |
/// | `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5 |
/// | `claude-3-5-sonnet-20241022` | Claude 3.5 Sonnet |
/// | `gemini-2.0-flash` | Gemini 2.0 Flash |
/// | `deepseek-chat` | DeepSeek V3 |
/// | `deepseek-reasoner` | DeepSeek R1 |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct PackyCodeExtension {
    base_url: String,
}

impl PackyCodeExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for PackyCodeExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn packycode_models() -> Vec<ModelInfo> {
    vec![
        // === OpenAI 系列 ===
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o，128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini，128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1".to_string(),
            description: "GPT-4.1，1M context".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1-mini".to_string(),
            description: "GPT-4.1 Mini，1M context".to_string(),
        },
        // === Claude 系列 ===
        ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            description: "Claude Sonnet 4.5，200K context".to_string(),
        },
        ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            description: "Claude 3.5 Sonnet，200K context".to_string(),
        },
        // === Google 系列 ===
        ModelInfo {
            id: "gemini-2.0-flash".to_string(),
            description: "Gemini 2.0 Flash，1M context".to_string(),
        },
        // === DeepSeek 系列 ===
        ModelInfo {
            id: "deepseek-chat".to_string(),
            description: "DeepSeek V3 — 对话模型".to_string(),
        },
        ModelInfo {
            id: "deepseek-reasoner".to_string(),
            description: "DeepSeek R1 — 推理模型".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for PackyCodeExtension {
    fn id(&self) -> &str {
        "packycode"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(packycode_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 20,
            initial_limit: 5,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<PackyCodeExtension> {
    Arc::new(PackyCodeExtension::new())
}
