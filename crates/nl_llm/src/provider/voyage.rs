use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, Modality, ModelResolver};
use crate::provider::extension::{EmbeddingData, ModelInfo, ProviderExtension, RerankResult};

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
        let normalized = model.trim().to_lowercase();
        match normalized.as_str() {
            "voyage3" => "voyage-3".to_string(),
            "voyage3lite" => "voyage-3-lite".to_string(),
            "voyagecode2" => "voyage-code-2".to_string(),
            "voyagelaw2" => "voyage-law-2".to_string(),
            "voyagefinance2" => "voyage-finance-2".to_string(),
            "voyagemultilingual2" => "voyage-multilingual-2".to_string(),
            "rerank2" => "rerank-2".to_string(),
            "rerank2lite" => "rerank-2-lite".to_string(),
            _ => model.to_string(),
        }
    }

    fn has_capability(&self, _model: &str, _capability: Capability) -> bool {
        // Voyage 的模型是 Embedding/Reranking 模型，不支持 CHAT/STREAMING/VISION/TOOLS
        false
    }

    fn max_context(&self, model: &str) -> usize {
        let resolved = self.resolve(model);
        if resolved.starts_with("voyage-3") {
            32_000
        } else if resolved.contains("code")
            || resolved.contains("finance")
            || resolved.contains("law")
        {
            16_000
        } else {
            8_000
        }
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max, 0)
    }

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        let resolved = self.resolve(model);
        if resolved.starts_with("rerank-") {
            Some((4.2, Modality::Text))
        } else {
            Some((4.0, Modality::Embedding))
        }
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
                description:
                    "General-purpose embedding model optimized for a wide range of use cases"
                        .to_string(),
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
            ModelInfo {
                id: "rerank-2".to_string(),
                description: "High-accuracy reranking model for search relevance".to_string(),
            },
            ModelInfo {
                id: "rerank-2-lite".to_string(),
                description: "Low-latency reranking model for online retrieval".to_string(),
            },
        ])
    }

    async fn embed(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
        model: &str,
        input: &[String],
    ) -> anyhow::Result<Vec<EmbeddingData>> {
        let req = http
            .post("https://api.voyageai.com/v1/embeddings")
            .json(&json!({
                "model": model,
                "input": input,
            }));
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();
        let value: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let message = value
                .get("detail")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    value
                        .get("error")
                        .and_then(|v| v.get("message"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("voyage embeddings request failed");
            return Err(anyhow::anyhow!("{}", message));
        }

        let data = value
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("voyage embeddings response missing data array"))?;

        Ok(data
            .iter()
            .map(|item| EmbeddingData {
                index: item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                embedding: item
                    .get("embedding")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|n| n.as_f64())
                            .map(|n| n as f32)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            })
            .collect())
    }

    async fn rerank(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
        model: &str,
        query: &str,
        documents: &[String],
        top_k: Option<usize>,
    ) -> anyhow::Result<Vec<RerankResult>> {
        let mut body = json!({
            "model": model,
            "query": query,
            "documents": documents,
        });
        if let Some(k) = top_k {
            body["top_k"] = json!(k);
        }

        let req = http.post("https://api.voyageai.com/v1/rerank").json(&body);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();
        let value: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let message = value
                .get("detail")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    value
                        .get("error")
                        .and_then(|v| v.get("message"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("voyage rerank request failed");
            return Err(anyhow::anyhow!("{}", message));
        }

        let candidates = value
            .get("data")
            .and_then(|v| v.as_array())
            .or_else(|| value.get("results").and_then(|v| v.as_array()))
            .ok_or_else(|| anyhow::anyhow!("voyage rerank response missing data/results array"))?;

        Ok(candidates
            .iter()
            .map(|item| RerankResult {
                index: item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                relevance_score: item
                    .get("relevance_score")
                    .or_else(|| item.get("score"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32,
                document: item
                    .get("document")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
            .collect())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(5)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Modality, ModelResolver};

    use super::VoyageModelResolver;

    #[test]
    fn voyage_alias_and_modality_are_resolved() {
        let resolver = VoyageModelResolver::new();

        assert_eq!(resolver.resolve("voyage3"), "voyage-3");
        assert_eq!(resolver.resolve("rerank2"), "rerank-2");
        assert_eq!(
            resolver.intelligence_and_modality("rerank2"),
            Some((4.2, Modality::Text))
        );
        assert_eq!(
            resolver.intelligence_and_modality("voyage3"),
            Some((4.0, Modality::Embedding))
        );
    }
}
