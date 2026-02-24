//! OpenRouter 扩展实现
//!
//! OpenRouter 是一个 LLM API 聚合网关，支持多个后端提供商。

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

/// OpenRouter 扩展
pub struct OpenRouterExtension;

impl OpenRouterExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenRouterExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenRouter 模型列表 API 响应
#[derive(Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
}

#[async_trait]
impl ProviderExtension for OpenRouterExtension {
    fn id(&self) -> &str {
        "openrouter"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get("https://openrouter.ai/api/v1/models");
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenRouter models API failed ({}): {}", status, err));
        }

        let json: OpenRouterModelsResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse models response: {}", e))?;

        let models: Vec<ModelInfo> = json.data.into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                description: m.description
                    .or(m.name)
                    .unwrap_or_else(|| "OpenRouter model".to_string()),
            })
            .collect();

        Ok(models)
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<String>> {
        // OpenRouter 没有公开的余额查询 API
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // OpenRouter: 并发取决于后端提供商
        // 使用保守的默认值
        ConcurrencyConfig::new(10)
    }
}

pub fn extension() -> Arc<OpenRouterExtension> {
    Arc::new(OpenRouterExtension::new())
}
