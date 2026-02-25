use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// RightCode 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://www.right.codes/codex/v1";

/// RightCode 企业级 AI Agent 中转平台扩展
///
/// Right Code 涵盖 Claude Code、Codex、Gemini CLI、Grok Code 的统一接入与管理。
/// 兼容 OpenAI 协议。
///
/// ## 认证方式
///
/// 标准 `Authorization: Bearer <key>` 格式，密钥 `sk-` 前缀。
///
/// ## 支持的模型
///
/// | 模型 ID | 输入价格 | 输出价格 |
/// |---------|---------|---------|
/// | `gpt-5` | $1.25/M | $10.00/M |
/// | `gpt-5-codex` | $1.25/M | $10.00/M |
/// | `gpt-5-codex-mini` | $0.25/M | $2.00/M |
/// | `gpt-5.1` | $1.25/M | $10.00/M |
/// | `gpt-5.1-codex` | $1.25/M | $10.00/M |
/// | `gpt-5.1-codex-max` | $1.25/M | $10.00/M |
/// | `gpt-5.1-codex-mini` | $0.25/M | $2.00/M |
/// | `gpt-5.2` | $1.75/M | $14.00/M |
/// | `gpt-5.2-codex` | $1.75/M | $14.00/M |
/// | `gpt-5.2-high/medium/low/xhigh` | $1.75/M | $14.00/M |
/// | `gpt-5.3-codex` | $1.75/M | $14.00/M |
/// | `gpt-5.3-codex-high/medium/low/xhigh` | $1.75/M | $14.00/M |
///
/// > **注意**: 用户套餐决定可用模型范围，Codex 套餐仅支持 codex 系列
///
/// ## 并发策略
///
/// - 官方上限: 10 并发
/// - 初始并发: 3
pub struct RightCodeExtension {
    base_url: String,
}

impl RightCodeExtension {
    pub fn new() -> Self {
        Self { base_url: DEFAULT_BASE_URL.to_string() }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for RightCodeExtension {
    fn default() -> Self { Self::new() }
}

fn rightcode_models() -> Vec<ModelInfo> {
    vec![
        // === GPT-5 系列 ===
        ModelInfo { id: "gpt-5".to_string(), description: "GPT-5，$1.25/$10.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5-codex".to_string(), description: "GPT-5 Codex，$1.25/$10.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5-codex-mini".to_string(), description: "GPT-5 Codex Mini，$0.25/$2.00 per M tokens".to_string() },
        // === GPT-5.1 系列 ===
        ModelInfo { id: "gpt-5.1".to_string(), description: "GPT-5.1，$1.25/$10.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.1-codex".to_string(), description: "GPT-5.1 Codex，$1.25/$10.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.1-codex-max".to_string(), description: "GPT-5.1 Codex Max，$1.25/$10.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.1-codex-mini".to_string(), description: "GPT-5.1 Codex Mini，$0.25/$2.00 per M tokens".to_string() },
        // === GPT-5.2 系列 ===
        ModelInfo { id: "gpt-5.2".to_string(), description: "GPT-5.2，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.2-codex".to_string(), description: "GPT-5.2 Codex，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.2-high".to_string(), description: "GPT-5.2 High，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.2-medium".to_string(), description: "GPT-5.2 Medium，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.2-low".to_string(), description: "GPT-5.2 Low，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.2-xhigh".to_string(), description: "GPT-5.2 XHigh，$1.75/$14.00 per M tokens".to_string() },
        // === GPT-5.3 系列 ===
        ModelInfo { id: "gpt-5.3-codex".to_string(), description: "GPT-5.3 Codex，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.3-codex-high".to_string(), description: "GPT-5.3 Codex High，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.3-codex-medium".to_string(), description: "GPT-5.3 Codex Medium，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.3-codex-low".to_string(), description: "GPT-5.3 Codex Low，$1.75/$14.00 per M tokens".to_string() },
        ModelInfo { id: "gpt-5.3-codex-xhigh".to_string(), description: "GPT-5.3 Codex XHigh，$1.75/$14.00 per M tokens".to_string() },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for RightCodeExtension {
    fn id(&self) -> &str { "rightcode" }

    async fn list_models(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(rightcode_models())
    }

    /// 获取账户余额或额度信息
    ///
    /// **注意**: RightCode 目前未提供公开的余额查询 API 文档。
    /// 如需查询余额，请访问平台控制台: https://right.codes
    async fn get_balance(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig { official_max: 10, initial_limit: 3, ..Default::default() }
    }
}

pub fn extension() -> Arc<RightCodeExtension> {
    Arc::new(RightCodeExtension::new())
}
