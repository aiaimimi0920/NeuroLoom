use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// Moonshot (月之暗面) 静态模型列表扩展
pub struct MoonshotExtension;

impl MoonshotExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MoonshotExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn moonshot_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "moonshot-v1-8k".to_string(),
            description: "Moonshot V1 8K — Standard model, 8K context".to_string(),
        },
        ModelInfo {
            id: "moonshot-v1-32k".to_string(),
            description: "Moonshot V1 32K — Long context model, 32K context".to_string(),
        },
        ModelInfo {
            id: "moonshot-v1-128k".to_string(),
            description: "Moonshot V1 128K — Extended context model, 128K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for MoonshotExtension {
    fn id(&self) -> &str {
        "moonshot"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(moonshot_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Moonshot: 使用保守的并发限制
        ConcurrencyConfig::new(10)
    }
}

pub fn extension() -> Arc<MoonshotExtension> {
    Arc::new(MoonshotExtension::new())
}
