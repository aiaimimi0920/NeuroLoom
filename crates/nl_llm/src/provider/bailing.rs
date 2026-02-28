use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

/// 百灵 (BaiLing) 默认基础 URL
///
/// 注意：官方文档使用 `/api/llm/v1`，cc-switch 使用 `/v1`。
/// 经实测，官方文档路径有效。
const DEFAULT_BASE_URL: &str = "https://api.tbox.cn/api/llm/v1";

/// BaiLing (百灵) 平台扩展
///
/// 蚂蚁集团百灵大模型，兼容 OpenAI 协议。
///
/// ## 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `Ling-1T` | 128K | 百灵旗舰模型 (响应为 Ling-max-2.0)，支持多模态 |
/// | `Ling-2.5-1T` | 128K | 百灵 2.5 版旗舰模型，支持多模态 |
/// | `Ling-flash` | 128K | 百灵 Flash 版，深度优化，支持多模态 |
/// | `Ling-mini` | 32K | 百灵轻量版，适合简单任务 |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct BaiLingExtension {
    #[allow(dead_code)]
    base_url: String,
}

impl BaiLingExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }
}

impl Default for BaiLingExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn bailing_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "Ling-1T".to_string(),
            description: "百灵旗舰模型，128K context，支持多模态".to_string(),
        },
        ModelInfo {
            id: "Ling-2.5-1T".to_string(),
            description: "百灵 2.5 版旗舰模型，128K context，支持多模态".to_string(),
        },
        ModelInfo {
            id: "Ling-flash".to_string(),
            description: "百灵 Flash 版，128K context，支持多模态".to_string(),
        },
        ModelInfo {
            id: "Ling-mini".to_string(),
            description: "百灵轻量版，32K context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for BaiLingExtension {
    fn id(&self) -> &str {
        "bailing"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(bailing_models())
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

pub fn extension() -> Arc<BaiLingExtension> {
    Arc::new(BaiLingExtension::new())
}
