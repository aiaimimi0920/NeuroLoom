use async_trait::async_trait;
use reqwest::Client;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, Modality, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// Voyage AI 模型解析器
///
/// Voyage AI 主要专注于文本嵌入 (Embedding) 和重排 (Reranking) 模型。
/// 这些模型不具备文本生成对话的能力。
pub struct VoyageModelResolver;

impl VoyageModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VoyageModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for VoyageModelResolver {
    fn resolve(&self, model: &str) -> String {
        model.to_string()
    }

    fn has_capability(&self, _model: &str, _capability: Capability) -> bool {
        // Voyage 的模型是 Embedding/Reranking 模型，不支持 CHAT/STREAMING/VISION/TOOLS
        false
    }

    fn max_context(&self, model: &str) -> usize {
        // 大致配置，常见如 voyage-3 支持 32k 的上下文
        if model.contains("voyage-3") {
            32_000
        } else if model.contains("code") || model.contains("finance") || model.contains("law") {
            16_000
        } else {
            4096
        }
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max, 0)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        // 标记为 Embedding 模态
        Some((4.0, Modality::Embedding))
    }
}

/// Voyage AI 扩展
pub struct VoyageExtension;

impl VoyageExtension {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VoyageExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderExtension for VoyageExtension {
    fn id(&self) -> &str {
        "voyage"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // Voyage 不提供 /v1/models 查询端点，静态提供已知模型
        Ok(vec![
            ModelInfo {
                id: "voyage-3".to_string(),
                description: "General-purpose embedding model optimized for a wide range of use cases".to_string(),
            },
            ModelInfo {
                id: "voyage-3-lite".to_string(),
                description: "Fast and lightweight embedding model".to_string(),
            },
            ModelInfo {
                id: "voyage-finance-2".to_string(),
                description: "Embedding model optimized for financial documents".to_string(),
            },
            ModelInfo {
                id: "voyage-multilingual-2".to_string(),
                description: "Multilingual embedding model".to_string(),
            },
            ModelInfo {
                id: "voyage-law-2".to_string(),
                description: "Embedding model optimized for legal documents".to_string(),
            },
            ModelInfo {
                id: "voyage-code-2".to_string(),
                description: "Embedding model optimized for source code".to_string(),
            },
        ])
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(5)
    }
}
