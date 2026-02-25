use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// DMXAPI 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://www.dmxapi.cn/v1";

/// DMXAPI 聚合平台扩展
///
/// DMXAPI 是国内 API 聚合平台，兼容 OpenAI 和 Anthropic 协议，
/// 提供 Claude、GPT 等多种模型的代理服务。
///
/// ## 认证方式
///
/// 标准 `Authorization: Bearer <key>` 格式。
///
/// ## 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `claude-sonnet-4-5-20250929` | 200K | Claude Sonnet 4.5 |
/// | `claude-opus-4-6` | 200K | Claude Opus 4.6 |
/// | `gpt-4o` | 128K | GPT-4o |
/// | `gpt-4o-mini` | 128K | GPT-4o Mini |
/// | `gpt-4.1` | 1M | GPT-4.1 |
/// | `gpt-4.1-mini` | 1M | GPT-4.1 Mini |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct DmxApiExtension {
    base_url: String,
}

impl DmxApiExtension {
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

impl Default for DmxApiExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn dmxapi_models() -> Vec<ModelInfo> {
    vec![
        // === Claude 系列 ===
        ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            description: "Claude Sonnet 4.5，200K context".to_string(),
        },
        ModelInfo {
            id: "claude-opus-4-6".to_string(),
            description: "Claude Opus 4.6，200K context".to_string(),
        },
        // === GPT 系列 ===
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o，128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini，128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1".to_string(),
            description: "GPT-4.1，1M context".to_string(),
        },
        ModelInfo {
            id: "gpt-4.1-mini".to_string(),
            description: "GPT-4.1 Mini，1M context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for DmxApiExtension {
    fn id(&self) -> &str {
        "dmxapi"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(dmxapi_models())
    }

    /// 获取账户余额或额度信息
    ///
    /// **注意**: DMXAPI 目前未提供公开的余额查询 API 文档。
    /// 如果您知道相关 API 端点，欢迎贡献实现。
    ///
    /// 可能在平台控制台查看余额: https://www.dmxapi.cn
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

pub fn extension() -> Arc<DmxApiExtension> {
    Arc::new(DmxApiExtension::new())
}
