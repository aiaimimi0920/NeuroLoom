use crate::provider::ProviderExtension;
use crate::{auth::traits::Authenticator, concurrency::ConcurrencyConfig, provider::ModelInfo};
use async_trait::async_trait;
use reqwest::Client;

/// 百川智能提供商特定扩展
pub struct BaichuanExtension {
    // 可以在这里持有 reqwest 客户端和认证信息等
}

impl BaichuanExtension {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BaichuanExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderExtension for BaichuanExtension {
    fn id(&self) -> &str {
        "baichuan"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "Baichuan4".to_string(),
                description: "Baichuan4 — 旗舰通用模型".to_string(),
            },
            ModelInfo {
                id: "Baichuan4-Turbo".to_string(),
                description: "Baichuan4-Turbo — 高频场景优化".to_string(),
            },
            ModelInfo {
                id: "Baichuan4-Air".to_string(),
                description: "Baichuan4-Air — 低成本快速模型".to_string(),
            },
            ModelInfo {
                id: "Baichuan3-Turbo".to_string(),
                description: "Baichuan3-Turbo — 稳定通用模型".to_string(),
            },
            ModelInfo {
                id: "Baichuan3-Turbo-128k".to_string(),
                description: "Baichuan3-Turbo-128k — 长上下文模型".to_string(),
            },
            ModelInfo {
                id: "Baichuan2-Turbo".to_string(),
                description: "Baichuan2-Turbo — 经典版本".to_string(),
            },
            ModelInfo {
                id: "Baichuan2-Turbo-192k".to_string(),
                description: "Baichuan2-Turbo-192k — 超长上下文模型".to_string(),
            },
        ])
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 保守默认值，避免在未知账号配额下触发频控。
        ConcurrencyConfig::new(20)
    }
}
