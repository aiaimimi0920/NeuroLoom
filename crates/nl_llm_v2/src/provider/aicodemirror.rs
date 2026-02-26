use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

/// AICodeMirror 默认基础 URL（ClaudeCode 通道，OpenAI 兼容）
const DEFAULT_BASE_URL: &str = "https://api.aicodemirror.com/api/claudecode/v1";

/// AICodeMirror AI 编程代理平台扩展
///
/// AICodeMirror 提供 Claude Code、Codex、Gemini 的统一代理服务。
/// ClaudeCode 通道同时支持 OpenAI 兼容格式和 Anthropic 原生格式。
///
/// ## 多线路选择
///
/// ### 全球高保线路
/// | 通道 | 端点 |
/// |------|------|
/// | ClaudeCode | `https://api.aicodemirror.com/api/claudecode` |
/// | Codex | `https://api.aicodemirror.com/api/codex/backend-api/codex` |
/// | Gemini | `https://api.aicodemirror.com/api/gemini` |
///
/// ### 国内优化线路
/// | 通道 | 端点 |
/// |------|------|
/// | ClaudeCode | `https://api.claudecode.net.cn/api/claudecode` |
/// | Codex | `https://api.claudecode.net.cn/api/codex/backend-api/codex` |
/// | Gemini | `https://api.claudecode.net.cn/api/gemini` |
///
/// ## 认证方式
///
/// - `Authorization: Bearer <key>`（OpenAI 兼容）
/// - `x-api-key: <key>`（Anthropic 原生）
/// - 密钥格式: `sk-ant-api03-...`
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct AiCodeMirrorExtension {
    base_url: String,
}

impl AiCodeMirrorExtension {
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

impl Default for AiCodeMirrorExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn aicodemirror_models() -> Vec<ModelInfo> {
    vec![
        // === Claude 4.6 ===
        ModelInfo {
            id: "claude-sonnet-4-6".to_string(),
            description: "Claude 4.6 Sonnet — 最新平衡模型，200K context".to_string(),
        },
        ModelInfo {
            id: "claude-opus-4-6".to_string(),
            description: "Claude 4.6 Opus — 最新旗舰模型，200K context".to_string(),
        },
        // === Claude 4.5 ===
        ModelInfo {
            id: "claude-sonnet-4-5-20250929".to_string(),
            description: "Claude Sonnet 4.5，200K context".to_string(),
        },
        ModelInfo {
            id: "claude-haiku-4-5-20251001".to_string(),
            description: "Claude 4.5 Haiku — 快速高效，200K context".to_string(),
        },
        // === Claude 4 ===
        ModelInfo {
            id: "claude-opus-4-20250514".to_string(),
            description: "Claude 4 Opus — 旗舰模型，200K context".to_string(),
        },
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            description: "Claude 4 Sonnet，200K context".to_string(),
        },
        // === Claude 3.7 ===
        ModelInfo {
            id: "claude-3-7-sonnet-20250219".to_string(),
            description: "Claude 3.7 Sonnet — 扩展思考，200K context".to_string(),
        },
        // === Claude 3.5 ===
        ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            description: "Claude 3.5 Sonnet，200K context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for AiCodeMirrorExtension {
    fn id(&self) -> &str {
        "aicodemirror"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(aicodemirror_models())
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

pub fn extension() -> Arc<AiCodeMirrorExtension> {
    Arc::new(AiCodeMirrorExtension::new())
}
