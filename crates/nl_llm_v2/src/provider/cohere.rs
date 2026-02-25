use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// Cohere AI 扩展
///
/// Cohere 提供 Command 系列模型，支持文本生成、对话、翻译和推理。
///
/// ## API 端点
///
/// - **原生 API**: `https://api.cohere.com/v2/chat`
/// - **OpenAI 兼容**: `https://api.cohere.com/compatibility/v1/chat/completions`
/// - **认证**: `Authorization: Bearer <key>`
///
/// ## 支持的模型
///
/// | 模型 ID | 说明 |
/// |---------|------|
/// | `command-a-03-2025` | Command A — 最新旗舰模型 |
/// | `command-a-vision-07-2025` | Command A Vision — 支持图像输入 |
/// | `command-a-reasoning-08-2025` | Command A Reasoning — 推理增强 |
/// | `command-a-translate-08-2025` | Command A Translate — 翻译专用 |
/// | `command-r-plus-08-2024` | Command R+ — 强力模型 |
/// | `command-r-08-2024` | Command R — 平衡模型 |
/// | `command-r7b-12-2024` | Command R 7B — 轻量级 |
///
/// ## 密钥类型
///
/// - **生产密钥**: 付费使用，无速率限制
/// - **试用密钥**: 免费，有速率限制
///
/// ## 并发策略
///
/// - 试用: 20 RPM
/// - 生产: 10,000 RPM
/// - 初始: 5
pub struct CohereExtension;

impl CohereExtension {
    pub fn new() -> Self { Self }
}

impl Default for CohereExtension {
    fn default() -> Self { Self::new() }
}

fn cohere_models() -> Vec<ModelInfo> {
    vec![
        // === Command A 系列 (最新) ===
        ModelInfo { id: "command-a-03-2025".to_string(), description: "Command A — 最新旗舰模型，对话与代码生成".to_string() },
        ModelInfo { id: "command-a-vision-07-2025".to_string(), description: "Command A Vision — 支持图像输入".to_string() },
        ModelInfo { id: "command-a-reasoning-08-2025".to_string(), description: "Command A Reasoning — 推理增强".to_string() },
        ModelInfo { id: "command-a-translate-08-2025".to_string(), description: "Command A Translate — 多语言翻译".to_string() },
        // === Command R+ 系列 ===
        ModelInfo { id: "command-r-plus-08-2024".to_string(), description: "Command R+ — 强力模型,128K context".to_string() },
        ModelInfo { id: "command-r-08-2024".to_string(), description: "Command R — 平衡模型，128K context".to_string() },
        // === 轻量级 ===
        ModelInfo { id: "command-r7b-12-2024".to_string(), description: "Command R 7B — 轻量快速".to_string() },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for CohereExtension {
    fn id(&self) -> &str { "cohere" }

    async fn list_models(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(cohere_models())
    }

    async fn get_balance(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig { official_max: 20, initial_limit: 5, ..Default::default() }
    }
}

pub fn extension() -> Arc<CohereExtension> {
    Arc::new(CohereExtension::new())
}
