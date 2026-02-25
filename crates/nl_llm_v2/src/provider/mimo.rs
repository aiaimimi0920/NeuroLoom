use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// MiMo 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.xiaomimimo.com/v1";

/// Xiaomi MiMo 平台扩展
///
/// 小米 MiMo 大模型，兼容 OpenAI 协议。
///
/// ## 认证方式
///
/// 支持两种认证头格式：
/// - `api-key: <key>`（官方文档推荐）
/// - `Authorization: Bearer <key>`（标准 OpenAI 兼容，实测可用）
///
/// ## 支持的模型
///
/// | 模型 ID | 上下文 | 能力 | 说明 |
/// |---------|--------|------|------|
/// | `mimo-v2-flash` | 128K | CHAT, TOOLS, STREAMING, THINKING | 旗舰模型，支持思考模式 |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
/// - 算法: AIMD (加增乘减)
///
/// ## 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("mimo")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
pub struct MiMoExtension {
    base_url: String,
}

impl MiMoExtension {
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

impl Default for MiMoExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn mimo_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "mimo-v2-flash".to_string(),
            description: "MiMo V2 Flash — 小米旗舰模型，128K context，支持思考模式和工具调用".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for MiMoExtension {
    fn id(&self) -> &str {
        "mimo"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(mimo_models())
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
            official_max: 20,
            initial_limit: 5,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<MiMoExtension> {
    Arc::new(MiMoExtension::new())
}
