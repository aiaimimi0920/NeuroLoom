use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

/// Cubence 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.cubence.com/v1";

/// Cubence AI API Gateway 扩展
///
/// Cubence 是专业 AI 工具代理平台，支持 Claude Code、Codex、Gemini CLI 等。
/// 兼容 OpenAI 协议。
///
/// ## 认证方式
///
/// 标准 `Authorization: Bearer <key>` 格式，密钥 `sk-user-` 前缀。
///
/// ## 可用端点
///
/// | 端点 | 说明 |
/// |------|------|
/// | `https://api.cubence.com` | 默认推荐 |
/// | `https://api-dmit.cubence.com` | 备用线路 |
/// | `https://api-bwg.cubence.com` | 备用线路 |
/// | `https://api-cf.cubence.com` | Cloudflare 线路 |
///
/// ## 支持的模型
///
/// | 模型 ID | 说明 |
/// |---------|------|
/// | `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5 |
/// | `claude-3-5-sonnet-20241022` | Claude 3.5 Sonnet |
/// | `gpt-4o` | GPT-4o |
/// | `gpt-4o-mini` | GPT-4o Mini |
/// | `gemini-2.0-flash` | Gemini 2.0 Flash |
/// | `gemini-2.5-pro` | Gemini 2.5 Pro |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct CubenceExtension {
    base_url: String,
}

impl CubenceExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for CubenceExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn cubence_models() -> Vec<ModelInfo> {
    vec![
        // === Claude 系列 ===
        ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            description: "Claude Sonnet 4.5，200K context".to_string(),
        },
        ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            description: "Claude 3.5 Sonnet，200K context".to_string(),
        },
        // === OpenAI 系列 ===
        ModelInfo {
            id: "gpt-4o".to_string(),
            description: "GPT-4o，128K context".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            description: "GPT-4o Mini，128K context".to_string(),
        },
        // === Google 系列 ===
        ModelInfo {
            id: "gemini-2.0-flash".to_string(),
            description: "Gemini 2.0 Flash，1M context".to_string(),
        },
        ModelInfo {
            id: "gemini-2.5-pro".to_string(),
            description: "Gemini 2.5 Pro，1M context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for CubenceExtension {
    fn id(&self) -> &str {
        "cubence"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(cubence_models())
    }

    /// 获取账户余额或额度信息
    ///
    /// **注意**: Cubence 目前未提供公开的余额查询 API 文档。
    /// 如需查询余额，请访问平台控制台: https://cubence.com
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

pub fn extension() -> Arc<CubenceExtension> {
    Arc::new(CubenceExtension::new())
}
