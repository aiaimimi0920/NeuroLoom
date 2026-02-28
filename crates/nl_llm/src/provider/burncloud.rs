use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, Modality, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// BurnCloud 模型解析器
///
/// BurnCloud 是一个主要作为 API Hub/网关的代理服务（类似 NewAPI / OneAPI 架构）。
/// 它自身并不提供特定的模型，而是代理来自不同源的模型，并兼容 OpenAI 接口。
pub struct BurnCloudModelResolver;

impl BurnCloudModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BurnCloudModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for BurnCloudModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 作为代理，模型名称由用户传递并透明路由
        model.to_string()
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // BurnCloud 是代理服务，能力由后端真实模型决定；此处放行常见通用能力。
        let supported =
            Capability::CHAT | Capability::STREAMING | Capability::VISION | Capability::TOOLS;
        supported.contains(capability)
    }

    fn max_context(&self, model: &str) -> usize {
        // 根据模型名称推测上下文，或者提供一个较为宽泛的保守值
        let lower = model.to_lowercase();
        if lower.contains("32k") {
            32_768
        } else if lower.contains("128k") {
            131_072
        } else if lower.contains("200k") {
            200_000
        } else if lower.contains("claude-3") || lower.contains("gpt-4") {
            128_000
        } else {
            16_384 // 取一个折中值
        }
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max, 0)
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, Modality)> {
        // 代理无法单方面评估单一模型的智力，采取默认设置
        Some((3.5, Modality::Text))
    }
}

/// 兼容 OpenAI `/v1/models` 端点的响应数据定义
#[derive(Debug, Deserialize)]
struct BurnCloudModelsResponse {
    #[serde(default)]
    data: Vec<BurnCloudModelItem>,
}

#[derive(Debug, Deserialize)]
struct BurnCloudModelItem {
    id: String,
}

/// BurnCloud 拓展功能
///
/// 具备从代理端点动态拉取受支持的模型列表的功能。
pub struct BurnCloudExtension {
    base_url: String,
}

impl BurnCloudExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn models_url(&self) -> String {
        if self.base_url.ends_with("/v1") {
            format!("{}/models", self.base_url)
        } else {
            format!("{}/v1/models", self.base_url)
        }
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4o".to_string(),
                description: "Fallback model (GPT-4o)".to_string(),
            },
            ModelInfo {
                id: "claude-3-5-sonnet-20240620".to_string(),
                description: "Fallback model (Claude 3.5 Sonnet)".to_string(),
            },
            ModelInfo {
                id: "gemini-1.5-pro".to_string(),
                description: "Fallback model (Gemini 1.5 Pro)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for BurnCloudExtension {
    fn id(&self) -> &str {
        "burncloud"
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
                let bytes = resp.bytes().await?;
                if let Ok(payload) = serde_json::from_slice::<BurnCloudModelsResponse>(&bytes) {
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
                } else {
                    Ok(Self::fallback_models())
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eprintln!(
                    "[{}] /models 检索失败: HTTP {} - {}。应用备用模型。",
                    self.id(),
                    status,
                    body
                );
                Ok(Self::fallback_models())
            }
            Err(e) => {
                eprintln!("[{}] 网络错误: {}。应用备用模型。", self.id(), e);
                Ok(Self::fallback_models())
            }
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 本地服务或代理枢纽的并发能力通常取决于具体的池大小
        ConcurrencyConfig::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::BurnCloudExtension;

    #[test]
    fn models_url_handles_v1_suffix() {
        let ext = BurnCloudExtension::new("https://api.burn.hair/v1/");
        assert_eq!(ext.models_url(), "https://api.burn.hair/v1/models");
    }

    #[test]
    fn models_url_patches_plain_base() {
        let ext = BurnCloudExtension::new("https://api.burn.hair");
        assert_eq!(ext.models_url(), "https://api.burn.hair/v1/models");
    }
}
