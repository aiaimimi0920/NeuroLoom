use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::{BalanceStatus, QuotaStatus, QuotaType, BillingUnit};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

/// Z.AI（智谱 GLM 海外版）扩展
///
/// Z.AI 是智谱 AI 的海外版本，API 基于 OpenAI 兼容协议。
///
/// ## 基本信息
///
/// - 官网：https://z.ai
/// - API 端点：`https://api.z.ai/api/paas/v4`
/// - 认证方式：Bearer Token
///
/// ## 功能
///
/// - 动态获取模型列表（调用 `/models` 端点）
/// - 平台并发配置
/// - 余额查询支持
pub struct ZaiExtension {
    base_url: String,
}

impl ZaiExtension {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.z.ai/api/paas/v4".to_string(),
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// 构建模型列表 API URL
    fn build_models_url(&self) -> String {
        format!("{}/models", self.base_url.trim_end_matches('/'))
    }

    /// 构建余额查询 API URL
    fn build_balance_url(&self) -> String {
        format!("{}/users/info/balance", self.base_url.trim_end_matches('/'))
    }
}

impl Default for ZaiExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Z.AI 平台模型静态列表（作为 API 调用失败的兜底）
fn fallback_models() -> Vec<ModelInfo> {
    vec![
        // GLM-5 系列
        ModelInfo {
            id: "glm-5".to_string(),
            description: "GLM-5 — 智谱旗舰模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-5-flash".to_string(),
            description: "GLM-5 Flash — 快速模型，128K context".to_string(),
        },
        // GLM-4 系列
        ModelInfo {
            id: "glm-4".to_string(),
            description: "GLM-4 — 多模态模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-flash".to_string(),
            description: "GLM-4 Flash — 轻量快速模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-plus".to_string(),
            description: "GLM-4 Plus — 增强版模型，128K context".to_string(),
        },
        // GLM-4V 视觉模型
        ModelInfo {
            id: "glm-4v".to_string(),
            description: "GLM-4V — 视觉多模态模型，支持图像理解".to_string(),
        },
    ]
}

/// OpenAI 兼容格式的模型列表响应
#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
}

/// Z.AI 余额查询响应格式
#[derive(Debug, Deserialize)]
struct ZaiBalanceResponse {
    #[serde(default)]
    balance: Option<f64>,
    #[serde(default)]
    total_balance: Option<f64>,
    #[serde(default)]
    message: Option<String>,
}

#[async_trait]
impl ProviderExtension for ZaiExtension {
    fn id(&self) -> &str {
        "zai"
    }

    /// 获取可用模型列表
    ///
    /// 优先调用 Z.AI API 获取实际模型列表，失败时返回静态兜底列表。
    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let url = self.build_models_url();

        let req = http.get(&url);
        let req = auth.inject(req)?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<OpenAiModelsResponse>().await {
                    Ok(api_resp) => {
                        let models: Vec<ModelInfo> = api_resp.data.into_iter()
                            .map(|m| ModelInfo {
                                id: m.id,
                                description: m.owned_by
                                    .map(|o| format!("Provider: {}", o))
                                    .unwrap_or_else(|| "Z.AI GLM Model".to_string()),
                            })
                            .collect();

                        if models.is_empty() {
                            Ok(fallback_models())
                        } else {
                            Ok(models)
                        }
                    }
                    Err(_) => Ok(fallback_models())
                }
            }
            _ => Ok(fallback_models())
        }
    }

    /// 获取账户余额
    ///
    /// Z.AI 提供余额查询接口，返回账户剩余额度。
    async fn get_balance(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        let url = self.build_balance_url();

        let req = http.get(&url);
        let req = auth.inject(req)?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ZaiBalanceResponse>().await {
                    Ok(balance_resp) => {
                        if let Some(balance) = balance_resp.balance.or(balance_resp.total_balance) {
                            Ok(Some(BalanceStatus {
                                display: format!("余额: ${:.4}", balance),
                                quota_type: QuotaType::PaidOnly,
                                free: None,
                                paid: Some(QuotaStatus {
                                    unit: BillingUnit::Money { currency: "USD".to_string() },
                                    used: 0.0,
                                    total: None,
                                    remaining: Some(balance),
                                    remaining_ratio: None,
                                    resets: false,
                                    reset_at: None,
                                }),
                                has_free_quota: false,
                                should_deprioritize: balance < 1.0,
                                is_unavailable: false,
                            }))
                        } else if let Some(msg) = balance_resp.message {
                            Ok(Some(BalanceStatus::error(msg)))
                        } else {
                            Ok(Some(BalanceStatus::error("余额信息不可用")))
                        }
                    }
                    Err(_) => Ok(None)
                }
            }
            _ => Ok(None)
        }
    }

    /// 获取并发配置
    ///
    /// Z.AI 作为智谱海外版，并发限制与国内版类似。
    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 10,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 10,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<ZaiExtension> {
    Arc::new(ZaiExtension::new())
}
