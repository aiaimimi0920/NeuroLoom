use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use super::balance::BalanceStatus;

/// 获取到的模型信息
///
/// # 示例
///
/// ```
/// use nl_llm_v2::provider::extension::ModelInfo;
///
/// let info = ModelInfo {
///     id: "gpt-4o".to_string(),
///     description: "GPT-4o — Flagship multimodal model".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// 模型标识符
    pub id: String,

    /// 模型描述或能力标签
    pub description: String,
}

/// 扩展 API 接口：各大平台特有的周边能力
///
/// 各平台可能提供不同的扩展能力，如：
/// - 模型列表查询
/// - 余额/额度查询
/// - 并发配置
///
/// # 实现说明
///
/// 每个平台应实现此 trait，提供平台特有的扩展能力。
/// 对于不支持的功能，使用默认实现即可。
///
/// # 示例
///
/// ```
/// use async_trait::async_trait;
/// use nl_llm_v2::provider::extension::{ProviderExtension, ModelInfo};
/// use nl_llm_v2::concurrency::ConcurrencyConfig;
///
/// struct MyExtension;
///
/// #[async_trait]
/// impl ProviderExtension for MyExtension {
///     fn id(&self) -> &str { "my-platform" }
///
///     async fn list_models(&self, http: &reqwest::Client, auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>> {
///         // 调用平台 API 获取模型列表
///         Ok(vec![
///             ModelInfo {
///                 id: "my-model".to_string(),
///                 description: "My Model".to_string(),
///             }
///         ])
///     }
///
///     fn concurrency_config(&self) -> ConcurrencyConfig {
///         ConcurrencyConfig {
///             official_max: 10,
///             initial_limit: 5,
///             ..Default::default()
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait ProviderExtension: Send + Sync {
    /// 扩展能力标识
    ///
    /// 应与预设名称保持一致。
    fn id(&self) -> &str;

    /// 获取可用模型列表
    ///
    /// 优先调用平台 API 获取实际模型列表，
    /// 失败时可返回静态兜底列表。
    ///
    /// # 参数
    ///
    /// - `http`: HTTP 客户端，用于发送请求
    /// - `auth`: 认证器，用于注入认证信息
    async fn list_models(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator
    ) -> anyhow::Result<Vec<ModelInfo>>;

    /// 获取平台的余额或额度信息
    ///
    /// 返回结构化的 `BalanceStatus`，包含：
    /// - 免费额度状态（如有）
    /// - 付费余额状态（如有）
    /// - 是否还有免费额度可用
    /// - 是否应该降低优先级
    ///
    /// # 实现说明
    ///
    /// - 各平台应根据自身 API 返回结构化数据
    /// - 不支持查询的平台返回 `Ok(None)` 或使用 `BalanceStatus::unsupported()`
    /// - 查询失败返回 `Err(...)`
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// async fn get_balance(&self, http: &Client, auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
    ///     // 调用平台 API
    ///     let resp = http.get("https://api.example.com/balance")
    ///         .bearer_auth(auth.api_key())
    ///         .send()
    ///         .await?;
    ///
    ///     let data: BalanceResponse = resp.json().await?;
    ///
    ///     Ok(Some(BalanceStatus {
    ///         display: format!("余额: ${}", data.balance),
    ///         quota_type: QuotaType::PaidOnly,
    ///         free: None,
    ///         paid: Some(QuotaStatus {
    ///             unit: BillingUnit::Money { currency: "USD".into() },
    ///             used: data.used,
    ///             total: None,
    ///             remaining: Some(data.balance),
    ///             remaining_ratio: None,
    ///             resets: false,
    ///             reset_at: None,
    ///         }),
    ///         has_free_quota: false,
    ///         should_deprioritize: data.balance < 1.0,
    ///         is_unavailable: false,
    ///     }))
    /// }
    /// ```
    async fn get_balance(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator
    ) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None) // 默认不支持
    }

    /// 获取并发配置
    ///
    /// 返回该平台的官方最大并发数和推荐配置。
    /// 用于 `ConcurrencyController` 的初始化。
    ///
    /// # 默认配置
    ///
    /// ```ignore
    /// ConcurrencyConfig {
    ///     official_max: 10,
    ///     initial_limit: 5,
    ///     min_limit: 1,
    ///     max_limit: 10,
    ///     ..Default::default()
    /// }
    /// ```
    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::default()
    }
}
