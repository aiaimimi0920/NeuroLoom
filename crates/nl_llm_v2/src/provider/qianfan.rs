//! 百度千帆大模型平台 v2 扩展
//!
//! 千帆 v2 API 使用 OpenAI 兼容格式。
//!
//! ## API 端点
//!
//! - **OpenAI 兼容**: `https://qianfan.baidubce.com/v2/chat/completions`
//! - **认证**: `Authorization: Bearer <api_key>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 输入/输出价格 (元/千token) |
//! |---------|------|--------------------------|
//! | `ernie-4.5-turbo-128k` | ERNIE 4.5 Turbo — 最新旗舰 | ¥0.004/¥0.012 |
//! | `ernie-4.5-8k` | ERNIE 4.5 标准版 | ¥0.004/¥0.012 |
//! | `ernie-4.0-turbo-128k` | ERNIE 4.0 Turbo | ¥0.03/¥0.09 |
//! | `ernie-4.0-turbo-8k` | ERNIE 4.0 Turbo 8K | ¥0.03/¥0.09 |
//! | `ernie-3.5-128k` | ERNIE 3.5 — 性价比 | ¥0.001/¥0.002 |
//! | `ernie-3.5-8k` | ERNIE 3.5 8K | ¥0.001/¥0.002 |
//! | `ernie-speed-128k` | ERNIE Speed — 最快 | ¥0.001/¥0.002 |
//! | `ernie-speed-8k` | ERNIE Speed 8K | 免费 |
//! | `ernie-lite-128k` | ERNIE Lite — 轻量 | 免费 |
//! | `ernie-lite-8k` | ERNIE Lite 8K | 免费 |
//! | `ernie-tiny-8k` | ERNIE Tiny — 最小 | 免费 |
//!
//! ## 获取密钥
//!
//! 1. 注册百度智能云: https://cloud.baidu.com
//! 2. 进入千帆大模型平台: https://qianfan.cloud.baidu.com
//! 3. 创建应用 → 获取 API Key
//!
//! ## 并发策略
//!
//! - 免费模型: 5 QPS
//! - 付费模型: 根据套餐
//! - 初始: 3

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

pub struct QianfanExtension;

impl QianfanExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QianfanExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn qianfan_models() -> Vec<ModelInfo> {
    vec![
        // === ERNIE 4.5 系列（最新）===
        ModelInfo {
            id: "ernie-4.5-turbo-128k".to_string(),
            description: "ERNIE 4.5 Turbo 128K — 最新旗舰，¥0.004/¥0.012".to_string(),
        },
        ModelInfo {
            id: "ernie-4.5-8k".to_string(),
            description: "ERNIE 4.5 8K — 标准版，¥0.004/¥0.012".to_string(),
        },
        // === ERNIE 4.0 系列 ===
        ModelInfo {
            id: "ernie-4.0-turbo-128k".to_string(),
            description: "ERNIE 4.0 Turbo 128K — 强力模型，¥0.03/¥0.09".to_string(),
        },
        ModelInfo {
            id: "ernie-4.0-turbo-8k".to_string(),
            description: "ERNIE 4.0 Turbo 8K，¥0.03/¥0.09".to_string(),
        },
        // === ERNIE 3.5 系列 ===
        ModelInfo {
            id: "ernie-3.5-128k".to_string(),
            description: "ERNIE 3.5 128K — 性价比之选，¥0.001/¥0.002".to_string(),
        },
        ModelInfo {
            id: "ernie-3.5-8k".to_string(),
            description: "ERNIE 3.5 8K，¥0.001/¥0.002".to_string(),
        },
        // === ERNIE Speed/Lite/Tiny（免费）===
        ModelInfo {
            id: "ernie-speed-128k".to_string(),
            description: "ERNIE Speed 128K — 快速，免费".to_string(),
        },
        ModelInfo {
            id: "ernie-speed-8k".to_string(),
            description: "ERNIE Speed 8K — 快速，免费".to_string(),
        },
        ModelInfo {
            id: "ernie-lite-128k".to_string(),
            description: "ERNIE Lite 128K — 轻量，免费".to_string(),
        },
        ModelInfo {
            id: "ernie-lite-8k".to_string(),
            description: "ERNIE Lite 8K — 轻量，免费".to_string(),
        },
        ModelInfo {
            id: "ernie-tiny-8k".to_string(),
            description: "ERNIE Tiny 8K — 最小，免费".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for QianfanExtension {
    fn id(&self) -> &str {
        "qianfan"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(qianfan_models())
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
            official_max: 5,
            initial_limit: 3,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<QianfanExtension> {
    Arc::new(QianfanExtension::new())
}
