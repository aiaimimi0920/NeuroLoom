use async_trait::async_trait;
use reqwest::Client;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use std::sync::Arc;

/// DeepSeek 静态模型列表扩展
pub struct DeepSeekExtension;

impl DeepSeekExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeepSeekExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn deepseek_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "deepseek-chat".to_string(),
            description: "DeepSeek Chat — General purpose model, 64K context".to_string(),
        },
        ModelInfo {
            id: "deepseek-coder".to_string(),
            description: "DeepSeek Coder — Code specialist, 64K context".to_string(),
        },
        ModelInfo {
            id: "deepseek-reasoner".to_string(),
            description: "DeepSeek Reasoner — Advanced reasoning, 64K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for DeepSeekExtension {
    fn id(&self) -> &str {
        "deepseek"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(deepseek_models())
    }
}

pub fn extension() -> Arc<DeepSeekExtension> {
    Arc::new(DeepSeekExtension::new())
}
