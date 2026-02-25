use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

/// Nvidia NIM 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";

/// Nvidia NIM (NVIDIA Inference Microservices) 平台扩展
///
/// NVIDIA NIM 提供高性能 AI 推理 API，兼容 OpenAI 协议。
/// 支持 186+ 个开源和 NVIDIA 优化的模型。
///
/// ## 认证方式
///
/// 使用 NVIDIA API Key，格式 `nvapi-` 前缀，
/// 标准 `Authorization: Bearer <key>` 格式。
///
/// ## 模型获取
///
/// 通过 `GET /v1/models` API 动态获取完整模型列表（186+ 个模型），
/// 包括 Meta Llama、DeepSeek、Google Gemma、Mistral、Qwen 等系列。
///
/// ## 并发策略
///
/// - 官方上限: 10 并发
/// - 初始并发: 3
pub struct NvidiaExtension {
    base_url: String,
}

impl NvidiaExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for NvidiaExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Nvidia /v1/models API 响应
#[derive(Deserialize)]
struct NvidiaModelsResponse {
    data: Vec<NvidiaModel>,
}

#[derive(Deserialize)]
struct NvidiaModel {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
}

#[async_trait::async_trait]
impl ProviderExtension for NvidiaExtension {
    fn id(&self) -> &str {
        "nvidia"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let url = format!("{}/models", self.base_url);
        let req = http.get(&url);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Nvidia models API failed ({}): {}", status, err));
        }

        let json: NvidiaModelsResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse models response: {}", e))?;

        let models: Vec<ModelInfo> = json.data.into_iter()
            .map(|m| {
                let desc = m.owned_by
                    .map(|owner| format!("by {}", owner))
                    .unwrap_or_else(|| "NVIDIA NIM model".to_string());
                ModelInfo {
                    id: m.id,
                    description: desc,
                }
            })
            .collect();

        Ok(models)
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 10,
            initial_limit: 3,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<NvidiaExtension> {
    Arc::new(NvidiaExtension::new())
}
