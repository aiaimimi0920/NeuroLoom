//! Perplexity AI 预设
//!
//! Perplexity 提供带联网搜索能力的 AI 模型。
//!
//! ## API 端点
//!
//! - **OpenAI 兼容**: `https://api.perplexity.ai/chat/completions`
//! - **认证**: `Authorization: Bearer <api_key>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文 | 价格 |
//! |---------|------|--------|------|
//! | `sonar-pro` | 旗舰搜索（默认） | 200K | $3/$15 |
//! | `sonar` | 标准搜索 | 128K | $1/$1 |
//! | `sonar-reasoning-pro` | 深度推理+搜索 | 128K | $2/$8 |
//! | `sonar-reasoning` | 标准推理+搜索 | 128K | $1/$5 |
//! | `sonar-deep-research` | 深度研究 | 128K | $2/$8 |
//! | `r1-1776` | 离线推理(无搜索) | 128K | $2/$8 |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `perplexity` / `pplx` | sonar-pro |
//! | `sonar` | sonar |
//! | `reasoning` | sonar-reasoning-pro |
//! | `research` | sonar-deep-research |
//! | `r1` | r1-1776 |
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use nl_llm_v2::{LlmClient, PrimitiveRequest};
//!
//! let client = LlmClient::from_preset("perplexity")?
//!     .with_api_key("pplx-xxxx")
//!     .build();
//!
//! let req = PrimitiveRequest::single_user_message("2025年AI最新进展是什么？");
//! let resp = client.complete(&req).await?;
//! println!("{}", resp.content);
//! ```
//!
//! ## 获取密钥
//!
//! 1. 注册 https://www.perplexity.ai
//! 2. Settings → API → Generate API Key

use crate::client::ClientBuilder;
use crate::model::perplexity::PerplexityModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::perplexity::PerplexityExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

const PERPLEXITY_BASE_URL: &str = "https://api.perplexity.ai";

/// 创建 Perplexity AI 客户端构建器
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(PERPLEXITY_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(PerplexityModelResolver::new())
        .with_extension(Arc::new(PerplexityExtension::new()))
        .default_model("sonar-pro")
}
