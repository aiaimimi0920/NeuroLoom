use async_trait::async_trait;
use reqwest::Client;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

/// Anthropic (Claude) 静态模型列表扩展
///
/// 模型数据来源：CLIProxyAPI 参考项目 (GetClaudeModels)，截止 2026-02-24。
/// 不调用 API，使用静态列表（Anthropic 无公开 models 端点）。
pub struct AnthropicExtension;

impl AnthropicExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnthropicExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Claude 模型静态列表（来自 CLIProxyAPI 参考项目）
fn claude_models() -> Vec<ModelInfo> {
    vec![
        // Claude 4.6（最新）
        ModelInfo {
            id: "claude-opus-4-6".to_string(),
            description: "Claude 4.6 Opus — Premium model, 1M context, 128K output".to_string(),
        },
        ModelInfo {
            id: "claude-sonnet-4-6".to_string(),
            description: "Claude 4.6 Sonnet — Latest balanced model, 200K context".to_string(),
        },
        // Claude 4.5
        ModelInfo {
            id: "claude-opus-4-5-20251101".to_string(),
            description: "Claude 4.5 Opus — Premium intelligence, 200K context".to_string(),
        },
        ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            description: "Claude 4.5 Sonnet — Balanced model with thinking, 200K context".to_string(),
        },
        ModelInfo {
            id: "claude-haiku-4-5-20251001".to_string(),
            description: "Claude 4.5 Haiku — Fast and efficient, 200K context".to_string(),
        },
        // Claude 4
        ModelInfo {
            id: "claude-opus-4-20250514".to_string(),
            description: "Claude 4 Opus — Flagship model, 200K context, extended thinking".to_string(),
        },
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            description: "Claude 4 Sonnet — Balanced performance, 200K context".to_string(),
        },
        // Claude 3.7
        ModelInfo {
            id: "claude-3-7-sonnet-20250219".to_string(),
            description: "Claude 3.7 Sonnet — Extended thinking model, 200K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for AnthropicExtension {
    fn id(&self) -> &str {
        "anthropic"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(claude_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Claude 付费用户: 1,000 RPM (Tier 4+)
        // 使用保守值 50 作为默认
        ConcurrencyConfig::new(50)
    }
}

/// 返回 Arc 包装好的扩展实例（供 preset 使用）
pub fn extension() -> Arc<AnthropicExtension> {
    Arc::new(AnthropicExtension::new())
}
