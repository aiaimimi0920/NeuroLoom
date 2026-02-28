//! Perplexity AI 扩展
//!
//! Perplexity 提供带联网搜索能力的 AI 模型，擅长实时信息查询。
//!
//! ## API 端点
//!
//! - **OpenAI 兼容**: `https://api.perplexity.ai/chat/completions`
//! - **认证**: `Authorization: Bearer <api_key>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文 | 价格 ($/M tokens) |
//! |---------|------|--------|------------------|
//! | `sonar-pro` | Sonar Pro — 旗舰搜索模型 | 200K | $3/$15 |
//! | `sonar` | Sonar — 标准搜索模型 | 128K | $1/$1 |
//! | `sonar-reasoning-pro` | Sonar Reasoning Pro — 深度推理 | 128K | $2/$8 |
//! | `sonar-reasoning` | Sonar Reasoning — 标准推理 | 128K | $1/$5 |
//! | `sonar-deep-research` | Sonar Deep Research — 深度研究 | 128K | $2/$8 |
//! | `r1-1776` | R1-1776 — 离线推理模型 | 128K | $2/$8 |
//!
//! ## 特点
//!
//! - **联网搜索**: Sonar 系列模型自带实时联网搜索
//! - **引用来源**: 返回结果中包含搜索来源 URL
//! - **推理能力**: Reasoning 系列支持 Chain-of-Thought
//!
//! ## 获取密钥
//!
//! 1. 注册 https://www.perplexity.ai
//! 2. 进入 Settings → API → Generate API Key
//!
//! ## 并发策略
//!
//! - 默认: 50 RPM
//! - 初始: 5

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::perplexity::PERPLEXITY_MODEL_META;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

pub struct PerplexityExtension;

impl PerplexityExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PerplexityExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn perplexity_models() -> Vec<ModelInfo> {
    PERPLEXITY_MODEL_META
        .iter()
        .map(|meta| ModelInfo {
            id: meta.id.to_string(),
            description: format!(
                "{},{}K context,{}",
                meta.summary,
                meta.context / 1_000,
                meta.price_per_million
            ),
        })
        .collect()
}

#[async_trait::async_trait]
impl ProviderExtension for PerplexityExtension {
    fn id(&self) -> &str {
        "perplexity"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(perplexity_models())
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
            official_max: 50,
            initial_limit: 5,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<PerplexityExtension> {
    Arc::new(PerplexityExtension::new())
}
