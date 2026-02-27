use super::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use async_trait::async_trait;

/// 视频任务状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoTaskState {
    Submitted,
    Processing,
    Succeed,
    Failed,
}

/// 视频任务状态信息
#[derive(Debug, Clone)]
pub struct VideoTaskStatus {
    pub id: String,
    pub state: VideoTaskState,
    pub message: Option<String>,
    pub video_urls: Vec<String>,
}

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

/// Embedding 向量项
#[derive(Debug, Clone)]
pub struct EmbeddingData {
    pub index: usize,
    pub embedding: Vec<f32>,
}

/// Rerank 结果项
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f32,
    pub document: Option<String>,
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
        auth: &mut dyn Authenticator,
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
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None) // 默认不支持
    }

    /// 文本向量化（Embedding）
    async fn embed(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
        _model: &str,
        _input: &[String],
    ) -> anyhow::Result<Vec<EmbeddingData>> {
        Err(anyhow::anyhow!("embed not supported for this provider"))
    }

    /// 文本重排（Rerank）
    async fn rerank(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
        _model: &str,
        _query: &str,
        _documents: &[String],
        _top_k: Option<usize>,
    ) -> anyhow::Result<Vec<RerankResult>> {
        Err(anyhow::anyhow!("rerank not supported for this provider"))
    }

    /// 提交异步视频生成任务（如可灵 Kling、Luma 等）
    ///
    /// # 返回
    ///
    /// - `Ok(task_id)`: 任务提交成功，返回任务 ID
    async fn submit_video_task(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
        _req: &crate::primitive::PrimitiveRequest,
    ) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "submit_video_task not supported for this provider"
        ))
    }

    /// 查询异步视频生成任务状态
    ///
    /// # 参数
    ///
    /// - `task_id`: 通过 `submit_video_task` 获取的任务 ID
    async fn fetch_video_task(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
        _task_id: &str,
    ) -> anyhow::Result<VideoTaskStatus> {
        Err(anyhow::anyhow!(
            "fetch_video_task not supported for this provider"
        ))
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
