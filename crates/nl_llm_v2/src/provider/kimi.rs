use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};

/// Kimi (Moonshot AI) 扩展 — 静态模型列表
pub struct KimiExtension;

impl KimiExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProviderExtension for KimiExtension {
    fn id(&self) -> &str {
        "kimi"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "k2".to_string(),
                description: "Kimi K2 — Moonshot AI flagship coding model (131K context)".to_string(),
            },
            ModelInfo {
                id: "k2-thinking".to_string(),
                description: "Kimi K2 Thinking — Extended reasoning model".to_string(),
            },
            ModelInfo {
                id: "k2.5".to_string(),
                description: "Kimi K2.5 — Latest model with improved capabilities".to_string(),
            },
        ])
    }
}
