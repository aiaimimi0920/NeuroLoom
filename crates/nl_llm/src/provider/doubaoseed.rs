use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

/// 火山引擎 ARK 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://ark.cn-beijing.volces.com/api/v3";

/// DouBaoSeed (字节跳动 · 豆包) 平台扩展
///
/// 火山引擎 ARK API，兼容 OpenAI 协议。
///
/// ## 核心特性
///
/// - **静态模型列表**: 使用精选的优质模型列表
/// - **并发控制**: 火山引擎提供较高的并发余量
///
/// ## 模型说明
///
/// | 类别 | 模型 | 说明 |
/// |------|------|------|
/// | Seed 2.0 | doubao-seed-2-0-pro | 旗舰通用模型，128K |
/// | Seed 2.0 | doubao-seed-2-0-code | 编码专用模型 |
/// | Seed 1.6 | doubao-seed-1-6-lite | 轻量模型，64K |
/// | Pro | doubao-pro-32k/128k | 标准模型 |
/// | Thinking | doubao-1-5-thinking-pro | 推理思考模型 |
pub struct DouBaoSeedExtension {
    #[allow(dead_code)]
    base_url: String,
}

impl DouBaoSeedExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }
}

impl Default for DouBaoSeedExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn doubaoseed_models() -> Vec<ModelInfo> {
    vec![
        // === Seed 2.0 系列 ===
        ModelInfo {
            id: "doubao-seed-2-0-pro-260215".to_string(),
            description: "DouBao Seed 2.0 Pro — 旗舰通用模型，支持多模态，128K context".to_string(),
        },
        ModelInfo {
            id: "doubao-seed-2-0-code-preview-latest".to_string(),
            description: "DouBao Seed 2.0 Code — 编码专用模型，128K context".to_string(),
        },
        // === Seed 1.6 系列 ===
        ModelInfo {
            id: "doubao-seed-1-6-lite-250115".to_string(),
            description: "DouBao Seed 1.6 Lite — 轻量模型，支持多模态，64K context".to_string(),
        },
        // === Pro 系列 ===
        ModelInfo {
            id: "doubao-pro-32k-241215".to_string(),
            description: "DouBao Pro 32K — 标准模型，支持多模态，32K context".to_string(),
        },
        ModelInfo {
            id: "doubao-pro-128k-241215".to_string(),
            description: "DouBao Pro 128K — 标准模型，支持多模态，128K context".to_string(),
        },
        // === 思考模型 ===
        ModelInfo {
            id: "doubao-1-5-thinking-pro-250415".to_string(),
            description: "DouBao 1.5 Thinking Pro — 推理思考模型，128K context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for DouBaoSeedExtension {
    fn id(&self) -> &str {
        "doubaoseed"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(doubaoseed_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // 火山引擎 ARK 暂无公开的余额查询 API
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 50,
            initial_limit: 10,
            min_limit: 2,
            max_limit: 60,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<DouBaoSeedExtension> {
    Arc::new(DouBaoSeedExtension::new())
}
