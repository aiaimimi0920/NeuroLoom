use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// AI Proxy 模型解析器
///
/// AI Proxy 作为一个兼容了 OpenAI 的聚合代理平台，
/// 支持成百上千种模型（如 claude, openai, gemini 等），
/// 所以统一放行对应的 CHAT 和 STREAMING 能力，
/// 默认兜底使用 gpt-4o。
pub struct AiProxyModelResolver;

impl AiProxyModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AiProxyModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AiProxyModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "gpt-4o".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // AI Proxy 聚合各类模型，这里全部放行对讲和流式，以兼顾各种模型
        capability.contains(Capability::CHAT) || capability.contains(Capability::STREAMING)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 动态池，一律给一个大的通用上下文边界
        128_000
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (100_000, 28_000)
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, crate::model::resolver::Modality)> {
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

/// AI Proxy 的 `/models` 兼容 OpenAI 响应结构。
#[derive(Debug, Deserialize)]
struct AiProxyModelsResponse {
    #[serde(default)]
    data: Vec<AiProxyModelItem>,
}

#[derive(Debug, Deserialize)]
struct AiProxyModelItem {
    id: String,
}

/// AI Proxy 扩展：优先动态拉取模型列表，失败时回退到静态兜底。
pub struct AiProxyExtension {
    base_url: String,
}

impl AiProxyExtension {
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
                description: "Fallback model (OpenAI-compatible)".to_string(),
            },
            ModelInfo {
                id: "gpt-4o-mini".to_string(),
                description: "Fallback model (OpenAI-compatible)".to_string(),
            },
            ModelInfo {
                id: "claude-3-5-sonnet".to_string(),
                description: "Fallback model (AI Proxy aggregated)".to_string(),
            },
            ModelInfo {
                id: "gemini-2.0-flash".to_string(),
                description: "Fallback model (AI Proxy aggregated)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for AiProxyExtension {
    fn id(&self) -> &str {
        "aiproxy"
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
                let payload: AiProxyModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| anyhow::anyhow!("aiproxy /models 响应解析失败: {}", e))?;

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
                    "[aiproxy] /models 请求失败 ({}): {}，使用静态兜底列表",
                    status, body
                );
                Ok(Self::fallback_models())
            }
            Err(e) => {
                eprintln!("[aiproxy] /models 网络错误: {}，使用静态兜底列表", e);
                Ok(Self::fallback_models())
            }
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 聚合平台上游能力波动更大，使用更保守的默认并发。
        ConcurrencyConfig::new(50)
    }
}
