use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// FastGPT 扩展定义
///
/// FastGPT 是一个基于 LLM 大语言模型的知识库问答系统。
/// 它提供了与 OpenAI 兼容的 API 接口，因此可以直接复用 OpenAI 的协议处理逻辑。
pub struct FastGptExtension {}

impl Default for FastGptExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl FastGptExtension {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

fn default_model() -> ModelInfo {
    ModelInfo {
        id: "fastgpt-default".to_string(),
        description: "FastGPT 绑定的默认应用模型".to_string(),
    }
}

#[async_trait]
impl ProviderExtension for FastGptExtension {
    fn id(&self) -> &str {
        "fastgpt"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // FastGPT 大多数部署兼容 OpenAI 的 /models 端点。
        // 若部署未开放该端点，则回退到默认模型以维持可用性。
        let req = http.get("https://api.fastgpt.in/api/v1/models");
        let req = auth.inject(req)?;
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Ok(vec![default_model()]);
        }

        let payload: OpenAiModelsResponse = match resp.json().await {
            Ok(data) => data,
            Err(_) => return Ok(vec![default_model()]),
        };

        let models: Vec<ModelInfo> = payload
            .data
            .into_iter()
            .map(|m| ModelInfo {
                description: format!("FastGPT model: {}", m.id),
                id: m.id,
            })
            .collect();

        if models.is_empty() {
            Ok(vec![default_model()])
        } else {
            Ok(models)
        }
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // FastGPT 暂未标准化统一的余额查询接口
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // FastGPT 常用于私有部署，默认值保守一些，避免给实例造成突刺压力。
        ConcurrencyConfig::new(8)
    }
}

pub fn extension() -> Arc<FastGptExtension> {
    Arc::new(FastGptExtension::new())
}
