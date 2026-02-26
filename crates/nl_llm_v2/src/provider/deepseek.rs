use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::{BalanceStatus, BillingUnit, QuotaStatus, QuotaType};
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

/// DeepSeek 默认 API 基础 URL
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

/// DeepSeek 平台扩展
///
/// DeepSeek 是一家中国 AI 公司，提供高性价比的 LLM API 服务。
///
/// ## 核心特性
///
/// - **余额查询**: 通过 `/user/balance` 端点获取账户余额
/// - **静态模型列表**: API 不提供动态模型列表，使用静态配置
/// - **并发控制**: 基于官方 RPM 限制配置（免费 60 RPM，付费 500 RPM）
///
/// ## 模型说明
///
/// DeepSeek 当前 API 仅暴露两个端点名，后端实际运行 DeepSeek-V3.2 系列：
/// - `deepseek-chat`: 通用对话模型，支持工具调用
/// - `deepseek-reasoner`: 深度推理模型，支持链式思考
///
/// ## 余额查询
///
/// DeepSeek 提供详细的余额信息，包括：
/// - 总余额
/// - 赠送余额
/// - 充值余额
/// - 货币类型
///
/// ## 并发策略
///
/// 采用保守的并发配置：
/// - 官方最大并发：20（保守估计）
/// - 初始并发：10（避免触发限流）
/// - 使用 AIMD 算法动态调节
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("deepseek")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .with_concurrency()
///     .build();
///
/// // 查询余额
/// let balance = client.get_balance().await?;
/// println!("余额: {:?}", balance);
/// ```
pub struct DeepSeekExtension {
    /// API 基础 URL（不含 /v1），用于构建余额查询等管理端点
    base_url: String,
}

impl DeepSeekExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL（用于代理场景）
    ///
    /// 传入的 URL 应不含 `/v1` 后缀，例如 `https://api.deepseek.com`
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        // 去除可能带的 /v1 后缀
        self.base_url = url
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .to_string();
        self
    }

    /// 构建余额查询 URL
    fn build_balance_url(&self) -> String {
        format!("{}/user/balance", self.base_url)
    }
}

impl Default for DeepSeekExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn deepseek_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "deepseek-chat".to_string(),
            description: "DeepSeek-V3.2 Chat — 通用对话，非推理模式，64K context".to_string(),
        },
        ModelInfo {
            id: "deepseek-reasoner".to_string(),
            description: "DeepSeek-V3.2 Reasoner — 深度推理，链式思考模式，64K context".to_string(),
        },
    ]
}

/// DeepSeek 余额 API 响应结构
#[derive(Deserialize)]
struct DeepSeekBalanceResponse {
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

#[async_trait]
impl ProviderExtension for DeepSeekExtension {
    fn id(&self) -> &str {
        "deepseek"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(deepseek_models())
    }

    async fn get_balance(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        let url = self.build_balance_url();
        let req = http.get(&url);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Ok(Some(BalanceStatus::error(format!(
                "API 错误 ({}): {}",
                status, err_text
            ))));
        }

        let json: DeepSeekBalanceResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse balance response: {}", e))?;

        if !json.is_available {
            return Ok(Some(BalanceStatus {
                display: "账户不可用".to_string(),
                quota_type: QuotaType::Unknown,
                free: None,
                paid: None,
                has_free_quota: false,
                should_deprioritize: true,
                is_unavailable: true,
            }));
        }

        // 解析余额信息
        // DeepSeek 返回赠送余额 (granted) 和充值余额 (topped_up)
        let mut display_parts = Vec::new();
        let mut has_granted = false;
        let mut granted_balance = 0.0f64;
        let mut topped_up_balance = 0.0f64;

        for info in &json.balance_infos {
            display_parts.push(format!(
                "{}: 总额 {} (赠送 {} / 充值 {})",
                info.currency, info.total_balance, info.granted_balance, info.topped_up_balance
            ));

            // 尝试解析数值
            if let Ok(val) = info.granted_balance.parse::<f64>() {
                granted_balance += val;
                if val > 0.0 {
                    has_granted = true;
                }
            }
            if let Ok(val) = info.topped_up_balance.parse::<f64>() {
                topped_up_balance += val;
            }
        }

        let _total_balance = granted_balance + topped_up_balance;
        let has_free = granted_balance > 0.0;
        // 当赠送余额低于某个阈值时建议降优先级（这里设为 1.0 作为示例阈值）
        let should_deprioritize = has_granted && granted_balance < 1.0;

        Ok(Some(BalanceStatus {
            display: display_parts.join(", "),
            quota_type: if has_granted && topped_up_balance > 0.0 {
                QuotaType::Mixed
            } else if has_granted {
                QuotaType::FreeOnly
            } else {
                QuotaType::PaidOnly
            },
            free: if has_granted {
                Some(QuotaStatus {
                    unit: BillingUnit::Money {
                        currency: "CNY".to_string(),
                    },
                    used: 0.0, // DeepSeek 不返回已使用量
                    total: None,
                    remaining: Some(granted_balance),
                    remaining_ratio: None,
                    resets: false,
                    reset_at: None,
                })
            } else {
                None
            },
            paid: if topped_up_balance > 0.0 {
                Some(QuotaStatus {
                    unit: BillingUnit::Money {
                        currency: "CNY".to_string(),
                    },
                    used: 0.0,
                    total: None,
                    remaining: Some(topped_up_balance),
                    remaining_ratio: None,
                    resets: false,
                    reset_at: None,
                })
            } else {
                None
            },
            has_free_quota: has_free,
            should_deprioritize,
            is_unavailable: false,
        }))
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // DeepSeek: 60 RPM (免费), 500 RPM (付费)
        ConcurrencyConfig::new(20)
    }
}

pub fn extension() -> Arc<DeepSeekExtension> {
    Arc::new(DeepSeekExtension::new())
}
