use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// AIGoCode 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.aigocode.com/v1";

/// AIGoCode AI 编程助手平台扩展
///
/// AIGoCode 提供稳定高效的 AI 编程服务，支持 Claude、GPT、Gemini 等模型。
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
/// | `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5 |
/// | `claude-3-5-sonnet-20241022` | Claude 3.5 Sonnet |
/// | `gpt-4o` | GPT-4o |
/// | `gpt-4o-mini` | GPT-4o Mini |
/// | `gemini-2.0-flash` | Gemini 2.0 Flash |
/// | `deepseek-chat` | DeepSeek V3 |
/// | `deepseek-reasoner` | DeepSeek R1 |
///
/// ## 并发策略
///
/// - 官方上限: 5 (套餐限制)
/// - 初始并发: 3
pub struct AiGoCodeExtension {
    base_url: String,
}

impl AiGoCodeExtension {
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

impl Default for AiGoCodeExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn aigocode_models() -> Vec<ModelInfo> {
    vec![
        // === Claude 系列 ===
        ModelInfo { id: "claude-sonnet-4-5-20250929".to_string(), description: "Claude Sonnet 4.5，200K context".to_string() },
        ModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), description: "Claude 3.5 Sonnet，200K context".to_string() },
        // === OpenAI 系列 ===
        ModelInfo { id: "gpt-4o".to_string(), description: "GPT-4o，128K context".to_string() },
        ModelInfo { id: "gpt-4o-mini".to_string(), description: "GPT-4o Mini，128K context".to_string() },
        // === Google 系列 ===
        ModelInfo { id: "gemini-2.0-flash".to_string(), description: "Gemini 2.0 Flash，1M context".to_string() },
        // === DeepSeek 系列 ===
        ModelInfo { id: "deepseek-chat".to_string(), description: "DeepSeek V3 — 对话模型".to_string() },
        ModelInfo { id: "deepseek-reasoner".to_string(), description: "DeepSeek R1 — 推理模型".to_string() },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for AiGoCodeExtension {
    fn id(&self) -> &str { "aigocode" }

    async fn list_models(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(aigocode_models())
    }

    async fn get_balance(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig { official_max: 5, initial_limit: 3, ..Default::default() }
    }
}

pub fn extension() -> Arc<AiGoCodeExtension> {
    Arc::new(AiGoCodeExtension::new())
}
