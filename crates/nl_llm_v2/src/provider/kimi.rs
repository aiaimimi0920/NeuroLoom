use async_trait::async_trait;
use reqwest::Client;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::{BalanceStatus, QuotaStatus, QuotaType, BillingUnit};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;
use serde::Deserialize;

/// Kimi/Moonshot 默认 API 基础 URL
const DEFAULT_BASE_URL: &str = "https://api.moonshot.cn/v1";

/// 余额查询响应结构
#[derive(Debug, Deserialize)]
struct MoonshotBalanceResponse {
    code: i32,
    data: Option<MoonshotBalanceData>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct MoonshotBalanceData {
    available_balance: f64,
    cash_balance: f64,
    voucher_balance: f64,
}

/// Kimi (Moonshot) 平台扩展
///
/// 月之暗面提供的 Kimi API，支持通用大模型及专门为代码生成的 Kimi For Coding。
///
/// ## 核心特性
///
/// - **两种域名**: `api.moonshot.cn`（常规）和专用的 `api.kimi.com`（代码专用）
/// - **余额查询**: 支持查询账户可用余额、现金余额、代金券余额
/// - **并发控制**: 严谨的层级化并发控制策略
///
/// ## 模型说明
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `kimi-k2.5` | Chat, Tools, Streaming | 128K | 最新旗舰大模型 |
/// | `moonshot-v1-32k` | Chat, Tools, Streaming | 32K | 标准通用模型 |
/// | `kimi-for-coding` | Chat, Tools, Streaming | 128K | 代码专用模型 |
///
/// ## 并发策略
///
/// Kimi 有严格的速率控制：
/// - 官方最大并发：20（普通账号通常 10~30）
/// - 初始并发：5（保守起步）
/// - 最大探测上限：30
/// - 使用 AIMD 算法动态调节
///
/// ## 余额查询
///
/// Kimi 提供详细的余额信息：
/// - 可用余额（现金 + 代金券）
/// - 现金余额
/// - 代金券余额
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("kimi")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .with_concurrency()
///     .build();
///
/// // 查询余额
/// let balance = client.get_balance().await?;
/// println!("余额: {:?}", balance);
///
/// // 使用代码模型
/// let req = nl_llm_v2::PrimitiveRequest::single_user_message("写个排序算法")
///     .with_model("coding");  // 自动解析为 kimi-for-coding
/// ```
pub struct KimiExtension {
    /// API 基础 URL
    base_url: String,
}

impl KimiExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL（如使用专属 `api.kimi.com/v1`）
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for KimiExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Kimi 相关内置优质模型
fn kimi_models() -> Vec<ModelInfo> {
    vec![
        // === 通用基座模型 (Kimi k2.5) ===
        ModelInfo {
            id: "moonshot-v1-8k".to_string(),
            description: "Moonshot v1 8K — 基础轻量模型，拥有8K上下文".to_string(),
        },
        ModelInfo {
            id: "moonshot-v1-32k".to_string(),
            description: "Moonshot v1 32K — 平衡通用模型，拥有32K上下文".to_string(),
        },
        ModelInfo {
            id: "moonshot-v1-128k".to_string(),
            description: "Moonshot v1 128K — 旗舰长文本模型，最高支持128K".to_string(),
        },
        // 注意：k2.5 统一走这个
        ModelInfo {
            id: "kimi-k2.5".to_string(),
            description: "Kimi K2.5 — 最新优化版本，支持131072参数的超大杯模型".to_string(),
        },

        // === 编程专用模型 ===
        ModelInfo {
            id: "kimi-for-coding".to_string(),
            description: "Kimi For Coding — 极强代码生成能力的专项优化模型".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for KimiExtension {
    fn id(&self) -> &str {
        "kimi"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // Moonshot 确实有 /v1/models，但直接提供清洗过的列表体验更好
        Ok(kimi_models())
    }

    async fn get_balance(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // Moonshot 提供非标的 /v1/users/me/balance，可以通过扩展查询
        let url = format!("{}/users/me/balance", self.base_url);
        let req = http.get(&url);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Ok(Some(BalanceStatus::error(format!("API 错误 ({}): {}", status, err))));
        }

        let json: MoonshotBalanceResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("解析Kimi余额响应失败: {}", e))?;

        if json.code == 0 {
            if let Some(data) = json.data {
                let display = format!("可用余额: ¥{:.2} (现金: ¥{:.2}, 代金券: ¥{:.2})",
                    data.available_balance,
                    data.cash_balance,
                    data.voucher_balance);

                let has_voucher = data.voucher_balance > 0.0;
                let has_cash = data.cash_balance > 0.0;

                return Ok(Some(BalanceStatus {
                    display,
                    quota_type: if has_voucher && has_cash {
                        QuotaType::Mixed
                    } else if has_voucher {
                        QuotaType::FreeOnly // 代金券视为免费额度
                    } else {
                        QuotaType::PaidOnly
                    },
                    free: if has_voucher {
                        Some(QuotaStatus {
                            unit: BillingUnit::Money { currency: "CNY".to_string() },
                            used: 0.0,
                            total: None,
                            remaining: Some(data.voucher_balance),
                            remaining_ratio: None,
                            resets: false,
                            reset_at: None,
                        })
                    } else {
                        None
                    },
                    paid: if has_cash {
                        Some(QuotaStatus {
                            unit: BillingUnit::Money { currency: "CNY".to_string() },
                            used: 0.0,
                            total: None,
                            remaining: Some(data.cash_balance),
                            remaining_ratio: None,
                            resets: false,
                            reset_at: None,
                        })
                    } else {
                        None
                    },
                    has_free_quota: has_voucher,
                    should_deprioritize: data.available_balance < 1.0,
                    is_unavailable: false,
                }));
            }
        }

        Ok(Some(BalanceStatus::error(format!("API 返回异常: {}", json.message))))
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Kimi: 普通账号通常为最高10~30左右。非常严格的速率控制。
        ConcurrencyConfig {
            official_max: 20,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 30, 
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<KimiExtension> {
    Arc::new(KimiExtension::new())
}
