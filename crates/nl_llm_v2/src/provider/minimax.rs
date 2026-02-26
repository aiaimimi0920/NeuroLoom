use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;

/// MiniMax 英文站默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.minimax.io/v1";

/// MiniMax 平台扩展
///
/// 兼容 OpenAI 协议的大模型提供商。
///
/// ## 平台站点
///
/// - 英文站: `api.minimax.io` — 预设名 `minimax`
/// - 中国站: `api.minimaxi.com` — 预设名 `minimax_cn`
///
/// ## 支持的模型
///
/// | 模型 ID | 上下文 | 能力 | 说明 |
/// |---------|--------|------|------|
/// | `MiniMax-M2.5` | 200K | CHAT, TOOLS, STREAMING, THINKING | 旗舰模型，支持 CoT |
/// | `MiniMax-M2.5-highspeed` | 200K | CHAT, TOOLS, STREAMING | 旗舰高速版 |
/// | `MiniMax-M2.1` | 200K | CHAT, TOOLS, STREAMING, THINKING | 编程增强版，支持 CoT |
/// | `MiniMax-M2.1-highspeed` | 200K | CHAT, TOOLS, STREAMING | 编程增强高速版 |
/// | `MiniMax-M2` | 128K | CHAT, TOOLS, STREAMING | 标准模型 |
/// | `M2-her` | 128K | CHAT, TOOLS, STREAMING | 多角色扮演模型 |
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
/// // 使用预设创建客户端
/// let client = LlmClient::from_preset("minimax")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
pub struct MiniMaxExtension {
    base_url: String,
}

impl MiniMaxExtension {
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

impl Default for MiniMaxExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn minimax_models() -> Vec<ModelInfo> {
    vec![
        // === M2.5 系列 - 旗舰模型 ===
        ModelInfo {
            id: "MiniMax-M2.5".to_string(),
            description: "MiniMax M2.5 — 旗舰模型，200K context，支持 CoT 思考".to_string(),
        },
        ModelInfo {
            id: "MiniMax-M2.5-highspeed".to_string(),
            description: "MiniMax M2.5 Highspeed — 旗舰高速版，200K context".to_string(),
        },
        // === M2.1 系列 - 编程增强版 ===
        ModelInfo {
            id: "MiniMax-M2.1".to_string(),
            description: "MiniMax M2.1 — 编程增强版，200K context，支持 CoT 思考".to_string(),
        },
        ModelInfo {
            id: "MiniMax-M2.1-highspeed".to_string(),
            description: "MiniMax M2.1 Highspeed — 编程增强高速版，200K context".to_string(),
        },
        // === M2 系列 - 标准模型 ===
        ModelInfo {
            id: "MiniMax-M2".to_string(),
            description: "MiniMax M2 — 标准模型，128K context".to_string(),
        },
        // === M2-her - 多角色扮演 ===
        ModelInfo {
            id: "M2-her".to_string(),
            description: "MiniMax M2-Her — 多角色扮演模型，128K context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for MiniMaxExtension {
    fn id(&self) -> &str {
        "minimax"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(minimax_models())
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
