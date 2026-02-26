use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// 适用于“自定义渠道”的模型解析器
///
/// 自定义渠道不限制模型名，也不会进行预置模型校验。
/// 对于所有的模型，它都会直接透传，并盲目默认该模型支持 Chat 和 Streaming 功能。
pub struct CustomModelResolver;

impl CustomModelResolver {
    pub fn new() -> Self {
        Self
    }
}

impl ModelResolver for CustomModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 自定义渠道不做别名映射，原样透传
        model.to_string()
    }

    fn has_capability(&self, _model: &str, cap: Capability) -> bool {
        // 默认支持 Chat + Streaming；其他能力交由服务端判定
        let supported = Capability::CHAT | Capability::STREAMING;
        supported.contains(cap)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 泛用性地赋予一个最大的通用 Context Limit，依赖实际的自定义服务器配置起效
        128_000
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max * 3 / 4, max / 4)
    }
}

/// OpenAI 兼容 `GET /models` 返回结构。
#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    #[serde(default)]
    data: Vec<OpenAiModelItem>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelItem {
    id: String,
}

/// 自定义渠道扩展能力。
///
/// 优先尝试从 `GET {base_url}/models` 动态拉取模型列表，
/// 以匹配“自定义渠道模型不可预置”的设计目标。
pub struct CustomExtension {
    base_url: String,
}

impl CustomExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }
}

#[async_trait]
impl ProviderExtension for CustomExtension {
    fn id(&self) -> &str {
        "custom"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get(self.models_url());
        let req = auth.inject(req)?;
        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("custom /models 请求失败 ({}): {}", status, body);
        }

        let data: OpenAiModelsResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("custom /models 响应解析失败: {}", e))?;

        Ok(data
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                description: String::new(),
            })
            .collect())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 自定义渠道未知，使用保守默认值。
        ConcurrencyConfig::default()
    }
}
