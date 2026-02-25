use crate::concurrency::ConcurrencyConfig;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::balance::BalanceStatus;
use reqwest::Client;

/// Longcat AI 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.longcat.chat/openai/v1";

/// Longcat (Longcat AI) 平台扩展
///
/// 兼容 OpenAI 协议的提供商，主打 `LongCat-Flash-Chat` 模型。
///
/// ## 核心特性
///
/// - **API 形式**: 标准 OpenAI Completions 格式
/// - **特定模型**: 主打 `LongCat-Flash-Chat` 模型
/// - **静态模型列表**: 使用静态模型列表
///
/// ## 模型说明
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `LongCat-Flash-Chat` | Chat, Tools, Streaming | 128K | 基础模型 |
///
/// ## 并发策略
///
/// Longcat 是新兴平台，暂无详细并发限制说明：
/// - 官方最大并发：10（保守估计）
/// - 初始并发：3
/// - 使用 AIMD 算法动态调节
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("longcat")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .with_concurrency()
///     .build();
///
/// // 基础对话
/// let req = nl_llm_v2::PrimitiveRequest::single_user_message("你好")
///     .with_model("flash");  // 使用别名
/// ```
pub struct LongcatExtension {
    /// API 基础 URL
    base_url: String,
}

impl LongcatExtension {
    /// 创建新的 Longcat 扩展
    ///
    /// 默认使用官方网关地址。
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

impl Default for LongcatExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// Longcat 模型列表
fn longcat_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "LongCat-Flash-Chat".to_string(),
            description: "LongCat Flash Chat - Longcat AI 推出的基础语言模型".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for LongcatExtension {
    fn id(&self) -> &str {
        "longcat"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(longcat_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // 未提供获取余额的公开 API，暂时返回 None
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Longcat 是新兴平台，暂无详细并发限制说明，使用保守配置
        ConcurrencyConfig {
            official_max: 10,
            initial_limit: 3,
            ..Default::default()
        }
    }
}
