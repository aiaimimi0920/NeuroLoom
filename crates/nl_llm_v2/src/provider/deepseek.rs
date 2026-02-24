use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;
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
        self.base_url = url.trim_end_matches('/').trim_end_matches("/v1").to_string();
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
    ) -> anyhow::Result<Option<String>> {
        let url = self.build_balance_url();
        let req = http.get(&url);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("DeepSeek balance API failed ({}): {}", status, err_text));
        }

        let json: DeepSeekBalanceResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse balance response: {}", e))?;

        if !json.is_available {
            return Ok(Some("账户不可用".to_string()));
        }

        // 格式化余额信息
        let mut parts = Vec::new();
        for info in &json.balance_infos {
            parts.push(format!(
                "{}: 总额 {} (赠送 {} / 充值 {})",
                info.currency,
                info.total_balance,
                info.granted_balance,
                info.topped_up_balance
            ));
        }

        Ok(Some(parts.join(", ")))
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // DeepSeek: 60 RPM (免费), 500 RPM (付费)
        ConcurrencyConfig::new(20)
    }
}

pub fn extension() -> Arc<DeepSeekExtension> {
    Arc::new(DeepSeekExtension::new())
}
