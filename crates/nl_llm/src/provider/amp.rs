use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::site::base::amp::AmpConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

/// Sourcegraph Amp 平台扩展
///
/// Sourcegraph Amp (ampcode.com) 聚合了多个后端供应商的模型，
/// 包括 OpenAI、Anthropic、Google Gemini 等���
///
/// 此扩展提供：
/// - 动态模型列表（API 优先、静态兜底）
/// - 平台并发配置
///
/// ## 配置共享
///
/// 通过 `Arc<AmpConfig>` 与 `AmpSite` 共享 base_url 和 provider 配置，
/// 确保 URL 始终一致。
///
/// ## 并发策略
///
/// Amp 作为代理平台，并发能力取决于后端供应商：
/// - 官方最大并发：20（保守估计）
/// - 初始并发：10（避免触发限流）
/// - 使用 AIMD 算法动态调节
pub struct AmpExtension {
    config: Arc<AmpConfig>,
}

impl AmpExtension {
    pub fn new() -> Self {
        Self {
            config: Arc::new(AmpConfig::new()),
        }
    }

    /// 从共享配置创建（确保与 AmpSite 使用同一份配置）
    pub fn from_config(config: Arc<AmpConfig>) -> Self {
        Self { config }
    }
}

impl Default for AmpExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Amp 聚合的常见可用模型静态列表（作为 API 调用失败的兜底）
fn fallback_models() -> Vec<ModelInfo> {
    vec![
        // OpenAI 系列
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o — Flagship multimodal model, 128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini — Fast and affordable, 128K context".to_string(),
        },
        ModelInfo {
            id: "o1".to_string(),
            description: "o1 — Advanced reasoning model, 200K context".to_string(),
        },
        ModelInfo {
            id: "o1-mini".to_string(),
            description: "o1-mini — Fast reasoning model, 200K context".to_string(),
        },
        ModelInfo {
            id: "o3-mini".to_string(),
            description: "o3-mini — Latest reasoning model, 200K context".to_string(),
        },
        // Claude 系列
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            description: "Claude Sonnet 4 — Balanced performance and cost, 200K context"
                .to_string(),
        },
        ModelInfo {
            id: "claude-opus-4-20250514".to_string(),
            description: "Claude Opus 4 — Highest capability Claude model, 200K context"
                .to_string(),
        },
        // Gemini 系列
        ModelInfo {
            id: "gemini-2.5-pro".to_string(),
            description: "Gemini 2.5 Pro — Google's most capable model, 1M context".to_string(),
        },
        ModelInfo {
            id: "gemini-2.5-flash".to_string(),
            description: "Gemini 2.5 Flash — Fast Gemini model, 1M context".to_string(),
        },
    ]
}

/// Amp API 返回的模型列表响应格式
#[derive(Debug, Deserialize)]
struct AmpModelsResponse {
    data: Vec<AmpModel>,
}

#[derive(Debug, Deserialize)]
struct AmpModel {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
}

#[async_trait]
impl ProviderExtension for AmpExtension {
    fn id(&self) -> &str {
        "amp"
    }

    /// 获取可用模型列表
    ///
    /// 优先调用 Amp API 获取实际模型列表，失败时返回静态兜底列表。
    /// URL 通过共享的 AmpConfig 构建，与 AmpSite 保持一致。
    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let url = self.config.build_models_url();

        let req = http.get(&url);
        let req = auth.inject(req)?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<AmpModelsResponse>().await {
                    Ok(api_resp) => {
                        let models: Vec<ModelInfo> = api_resp
                            .data
                            .into_iter()
                            .map(|m| ModelInfo {
                                id: m.id,
                                description: m
                                    .owned_by
                                    .map(|o| format!("Provider: {}", o))
                                    .unwrap_or_else(|| "Available via Amp".to_string()),
                            })
                            .collect();

                        if models.is_empty() {
                            Ok(fallback_models())
                        } else {
                            Ok(models)
                        }
                    }
                    Err(_) => Ok(fallback_models()),
                }
            }
            _ => Ok(fallback_models()),
        }
    }

    /// 获取并发配置
    ///
    /// Amp 作为代理平台，并发能力取决于后端供应商。
    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 20,
            initial_limit: 10,
            min_limit: 2,
            max_limit: 20,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<AmpExtension> {
    Arc::new(AmpExtension::new())
}
