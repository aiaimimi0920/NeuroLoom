use async_trait::async_trait;
use reqwest::Client;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

/// Codex 静态模型列表扩展
///
/// 模型数据来源：CLIProxyAPI 参考项目，截止 2026-02-24。
/// Codex 无公开 models 端点，使用静态列表。
pub struct CodexExtension;

impl CodexExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn codex_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gpt-5.1-codex-max".to_string(),
            description: "GPT 5.1 Codex Max — Highest capability, 400K context, thinking xhigh".to_string(),
        },
        ModelInfo {
            id: "gpt-5.1-codex".to_string(),
            description: "GPT 5.1 Codex — Best for coding and agentic tasks, 400K context".to_string(),
        },
        ModelInfo {
            id: "gpt-5.1-codex-mini".to_string(),
            description: "GPT 5.1 Codex Mini — Faster and cheaper, 400K context".to_string(),
        },
        ModelInfo {
            id: "gpt-5-codex".to_string(),
            description: "GPT 5 Codex — Stable coding model, 400K context".to_string(),
        },
        ModelInfo {
            id: "gpt-5-codex-mini".to_string(),
            description: "GPT 5 Codex Mini — Faster GPT 5 variant, 400K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for CodexExtension {
    fn id(&self) -> &str {
        "codex"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(codex_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Codex: 较高并发支持
        ConcurrencyConfig::new(20)
    }
}

pub fn extension() -> Arc<CodexExtension> {
    Arc::new(CodexExtension::new())
}
