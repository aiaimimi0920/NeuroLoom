use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// NewAPI 模型解析器
///
/// NewAPI 是一个兼容 OpenAI 格式的中转代理协议，
/// 支持任意自定义模型名称，因此放行所有的请求，并赋予 CHAT 与 STREAMING 能力。
pub struct NewApiModelResolver;

impl NewApiModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NewApiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for NewApiModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 对于 NewAPI，保留原样模型名
        model.to_string()
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // NewAPI 是代理服务，理论上支持其背后模型的所有能力，默认全部放行
        capability.contains(Capability::CHAT) 
            || capability.contains(Capability::STREAMING) 
            || capability.contains(Capability::VISION) 
            || capability.contains(Capability::TOOLS)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 动态池不知道背后的真实模型上下文，给一个宽松值
        128_000
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (100_000, 28_000)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        // NewAPI 只用作转发，具体的能力取决于配置的模型
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

/// 兼容 OpenAI `/v1/models` 的响应结构
#[derive(Debug, Deserialize)]
struct NewApiModelsResponse {
    #[serde(default)]
    data: Vec<NewApiModelItem>,
}

#[derive(Debug, Deserialize)]
struct NewApiModelItem {
    id: String,
}

/// NewAPI 扩展：优先动态拉取模型列表
pub struct NewApiExtension {
    base_url: String,
}

impl NewApiExtension {
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
                description: "Fallback model (NewAPI compatible)".to_string(),
            },
            ModelInfo {
                id: "gpt-3.5-turbo".to_string(),
                description: "Fallback model (NewAPI compatible)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for NewApiExtension {
    fn id(&self) -> &str {
        "newapi"
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
                let payload: NewApiModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| anyhow::anyhow!("newapi /models 解析失败: {}", e))?;

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
                    "[newapi] /models 请求失败 ({}): {}，使用静态列表",
                    status, body
                );
                Ok(Self::fallback_models())
            }
            Err(e) => {
                eprintln!("[newapi] /models 网络错误: {}，使用静态列表", e);
                Ok(Self::fallback_models())
            }
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 自治度较高，适当增加并发以兼容多模型中转平台
        ConcurrencyConfig::new(50)
    }
}
