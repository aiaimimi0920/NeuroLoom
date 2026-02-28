use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

/// AiHubMix 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://aihubmix.com/v1";

/// AiHubMix 聚合平台扩展
///
/// AiHubMix 是一个 API 聚合平台，支持 OpenAI 和 Anthropic 协议，
/// 提供多种免费和付费模型。
///
/// ## 认证方式
///
/// 标准 `Authorization: Bearer <key>` 格式。
///
/// ## 免费模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `gpt-4o-free` | 1M | GPT-4o 免费版 (Azure) |
/// | `gpt-4.1-free` | 1M | GPT-4.1 免费版 |
/// | `gpt-4.1-mini-free` | 1M | GPT-4.1 Mini 免费版 |
/// | `gpt-4.1-nano-free` | 1M | GPT-4.1 Nano 免费版 |
/// | `gemini-2.0-flash-free` | 1M | Gemini 2.0 Flash 免费版 |
/// | `gemini-3-flash-preview-free` | 1M | Gemini 3 Flash 预览免费版 |
/// | `glm-4.7-flash-free` | - | GLM-4.7 Flash 免费版 |
/// | `step-3.5-flash-free` | 256K | Step 3.5 Flash 免费版 |
/// | `coding-glm-5-free` | - | Coding GLM 5 免费版 |
/// | `coding-glm-4.7-free` | - | Coding GLM 4.7 免费版 |
/// | `coding-glm-4.6-free` | - | Coding GLM 4.6 免费版 |
/// | `coding-minimax-m2-free` | - | Coding MiniMax M2 免费版 |
///
/// ## 付费模型（示例）
///
/// | 模型 ID | 说明 |
/// |---------|------|
/// | `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5 (200K) |
/// | `claude-opus-4-6` | Claude Opus 4.6 (200K) |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct AiHubMixExtension {
    base_url: String,
}

impl AiHubMixExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for AiHubMixExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn aihubmix_models() -> Vec<ModelInfo> {
    vec![
        // === 免费 GPT 模型 ===
        ModelInfo {
            id: "gpt-4o-free".to_string(),
            description: "GPT-4o 免费版 (Azure)，1M context".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1-free".to_string(),
            description: "GPT-4.1 免费版，1M context，72 TPS".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1-mini-free".to_string(),
            description: "GPT-4.1 Mini 免费版，1M context，59 TPS".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1-nano-free".to_string(),
            description: "GPT-4.1 Nano 免费版，1M context，110 TPS".to_string(),
        },
        // === 免费 Gemini 模型 ===
        ModelInfo {
            id: "gemini-2.0-flash-free".to_string(),
            description: "Gemini 2.0 Flash 免费版，1M context".to_string(),
        },
        ModelInfo {
            id: "gemini-3-flash-preview-free".to_string(),
            description: "Gemini 3 Flash 预览免费版，1M context".to_string(),
        },
        // === 免费 GLM 模型 ===
        ModelInfo {
            id: "glm-4.7-flash-free".to_string(),
            description: "GLM-4.7 Flash 免费版".to_string(),
        },
        ModelInfo {
            id: "coding-glm-5-free".to_string(),
            description: "Coding GLM 5 免费版".to_string(),
        },
        ModelInfo {
            id: "coding-glm-4.7-free".to_string(),
            description: "Coding GLM 4.7 免费版".to_string(),
        },
        ModelInfo {
            id: "coding-glm-4.6-free".to_string(),
            description: "Coding GLM 4.6 免费版".to_string(),
        },
        // === 其他免费模型 ===
        ModelInfo {
            id: "step-3.5-flash-free".to_string(),
            description: "Step 3.5 Flash 免费版，256K context".to_string(),
        },
        ModelInfo {
            id: "coding-minimax-m2-free".to_string(),
            description: "Coding MiniMax M2 免费版".to_string(),
        },
        // === 付费模型 ===
        ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            description: "Claude Sonnet 4.5，200K context (付费)".to_string(),
        },
        ModelInfo {
            id: "claude-opus-4-6".to_string(),
            description: "Claude Opus 4.6，200K context (付费)".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for AiHubMixExtension {
    fn id(&self) -> &str {
        "aihubmix"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(aihubmix_models())
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

pub fn extension() -> Arc<AiHubMixExtension> {
    Arc::new(AiHubMixExtension::new())
}
