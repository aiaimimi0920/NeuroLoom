use async_trait::async_trait;
use reqwest::Client;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use std::sync::Arc;

/// 智谱 AI (Zhipu) 静态模型列表扩展
pub struct ZhipuExtension;

impl ZhipuExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZhipuExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn zhipu_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "glm-4".to_string(),
            description: "GLM-4 — Flagship multimodal model, 128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-flash".to_string(),
            description: "GLM-4 Flash — Fast and efficient, 128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-plus".to_string(),
            description: "GLM-4 Plus — Enhanced multimodal, 128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-air".to_string(),
            description: "GLM-4 Air — Lightweight model, 128K context".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for ZhipuExtension {
    fn id(&self) -> &str {
        "zhipu"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(zhipu_models())
    }
}

pub fn extension() -> Arc<ZhipuExtension> {
    Arc::new(ZhipuExtension::new())
}
