use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;

/// 讯飞星火默认基础 URL (OpenAI 兼容端点)
const DEFAULT_BASE_URL: &str = "https://spark-api-open.xf-yun.com/v1";

/// 讯飞星火 (iFlytek Spark) 平台扩展
///
/// 科大讯飞推出的认知智能大模型，通过 OpenAI 兼容 HTTP REST API 访问。
///
/// ## 核心特性
///
/// - **OpenAI 兼容**: 标准 `/v1/chat/completions` 端点
/// - **认证方式**: `Authorization: Bearer <APIPassword>`（推荐）
///   - 兼容历史输入：`APIKey:APISecret`
/// - **静态模型列表**: 使用严格筛选的优质模型列表
///
/// ## 模型说明
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `4.0Ultra` | Chat, Tools, Streaming | 128K | 旗舰模型 |
/// | `max-32k` | Chat, Tools, Streaming | 32K | 长文本版 |
/// | `generalv3.5` | Chat, Tools, Streaming | 128K | Spark Max |
/// | `pro-128k` | Chat, Streaming | 128K | 长上下文 |
/// | `generalv3` | Chat, Streaming | 8K | Spark Pro |
/// | `lite` | Chat, Streaming | 4K | 免费轻量 |
///
/// ## 并发策略
///
/// 讯飞开放平台对免费用户并发较保守：
/// - 官方最大并发：10（保守估计）
/// - 初始并发：3
/// - 使用 AIMD 算法动态调节
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm_v2::LlmClient;
///
/// // 推荐：APIPassword
/// let client = LlmClient::from_preset("spark")
///     .expect("Preset should exist")
///     .with_spark_auth("your_api_password")
///     .with_concurrency()
///     .build();
///
/// let req = nl_llm_v2::PrimitiveRequest::single_user_message("你好")
///     .with_model("ultra");  // 自动解析为 4.0Ultra
/// ```
pub struct SparkExtension {
    /// API 基础 URL（目前仅用于对外配置透传，便于后续扩展动态模型探测接口）
    base_url: String,
}

impl SparkExtension {
    /// 创建新的讯飞星火扩展
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for SparkExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// 讯飞星火内置模型列表
fn spark_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "4.0Ultra".to_string(),
            description: "Spark 4.0 Ultra — 旗舰模型，128K 上下文，最强推理能力".to_string(),
        },
        ModelInfo {
            id: "max-32k".to_string(),
            description: "Spark Max-32K — 长文本版本，32K 上下文".to_string(),
        },
        ModelInfo {
            id: "generalv3.5".to_string(),
            description: "Spark Max (v3.5) — 通用旗舰版，128K 上下文".to_string(),
        },
        ModelInfo {
            id: "pro-128k".to_string(),
            description: "Spark Pro-128K — 长上下文专用，128K 上下文".to_string(),
        },
        ModelInfo {
            id: "generalv3".to_string(),
            description: "Spark Pro (v3) — 通用型，8K 上下文".to_string(),
        },
        ModelInfo {
            id: "lite".to_string(),
            description: "Spark Lite — 免费轻量模型，4K 上下文".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for SparkExtension {
    fn id(&self) -> &str {
        "spark"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let _ = &self.base_url;
        Ok(spark_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<super::balance::BalanceStatus>> {
        // 讯飞开放平台目前无公开的 REST 余额查询端点
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 10,
            initial_limit: 3,
            min_limit: 1,
            max_limit: 15,
            ..Default::default()
        }
    }
}
