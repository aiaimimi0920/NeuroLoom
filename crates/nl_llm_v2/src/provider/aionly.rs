use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct AiOnlyModelResolver;

impl AiOnlyModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AiOnlyModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AiOnlyModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "gpt-4o".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        let supported = Capability::CHAT | Capability::STREAMING;
        supported.contains(capability)
    }

    fn max_context(&self, _model: &str) -> usize {
        128_000
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (100_000, 28_000)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

#[derive(Debug, Deserialize)]
struct AiOnlyModelsResponse {
    #[serde(default)]
    data: Vec<AiOnlyModelItem>,
}

#[derive(Debug, Deserialize)]
struct AiOnlyModelItem {
    id: String,
}

pub struct AiOnlyExtension {
    base_url: String,
}

impl AiOnlyExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4o".to_string(),
                description: "OpenAI GPT-4o (AiOnly Proxy)".to_string(),
            },
            ModelInfo {
                id: "claude-3-5-sonnet-20240620".to_string(),
                description: "Claude 3.5 Sonnet (AiOnly Proxy)".to_string(),
            },
            ModelInfo {
                id: "gemini-1.5-pro".to_string(),
                description: "Gemini 1.5 Pro (AiOnly Proxy)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for AiOnlyExtension {
    fn id(&self) -> &str {
        "aionly"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get(self.models_url());
        let req = auth.inject(req)?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let payload: AiOnlyModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| anyhow::anyhow!("aionly /models 响应解析失败: {}", e))?;

                if payload.data.is_empty() {
                    Ok(Self::fallback_models())
                } else {
                    Ok(payload
                        .data
                        .into_iter()
                        .map(|m| ModelInfo {
                            id: m.id,
                            description: String::new(),
                        })
                        .collect())
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eprintln!(
                    "[aionly] /models 请求失败 ({}): {}，使用静态兜底列表",
                    status, body
                );
                Ok(Self::fallback_models())
            }
            Err(e) => {
                eprintln!("[aionly] /models 网络错误: {}，使用静态兜底列表", e);
                Ok(Self::fallback_models())
            }
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(20)
    }
}
