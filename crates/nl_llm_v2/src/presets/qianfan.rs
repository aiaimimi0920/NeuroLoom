//! 百度千帆大模型平台 v2 预设
//!
//! 千帆 v2 API 使用 OpenAI 兼容格式，提供 ERNIE 系列大模型服务。
//!
//! ## API 端点
//!
//! - **OpenAI 兼容**: `https://qianfan.baidubce.com/v2/chat/completions`
//! - **认证**: `Authorization: Bearer <api_key>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文长度 | 价格 (元/千token) |
//! |---------|------|-----------|------------------|
//! | `ernie-4.5-turbo-128k` | ERNIE 4.5 Turbo — 最新旗舰（默认） | 128K | ¥0.004/¥0.012 |
//! | `ernie-4.5-8k` | ERNIE 4.5 标准版 | 8K | ¥0.004/¥0.012 |
//! | `ernie-4.0-turbo-128k` | ERNIE 4.0 Turbo | 128K | ¥0.03/¥0.09 |
//! | `ernie-4.0-turbo-8k` | ERNIE 4.0 Turbo 8K | 8K | ¥0.03/¥0.09 |
//! | `ernie-3.5-128k` | ERNIE 3.5 — 性价比 | 128K | ¥0.001/¥0.002 |
//! | `ernie-3.5-8k` | ERNIE 3.5 8K | 8K | ¥0.001/¥0.002 |
//! | `ernie-speed-128k` | ERNIE Speed — 快速 | 128K | 免费 |
//! | `ernie-speed-8k` | ERNIE Speed 8K | 8K | 免费 |
//! | `ernie-lite-128k` | ERNIE Lite — 轻量 | 128K | 免费 |
//! | `ernie-lite-8k` | ERNIE Lite 8K | 8K | 免费 |
//! | `ernie-tiny-8k` | ERNIE Tiny — 最小 | 8K | 免费 |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `qianfan` / `ernie` / `文心` | ernie-4.5-turbo-128k |
//! | `4.5` | ernie-4.5-turbo-128k |
//! | `4.0` | ernie-4.0-turbo-128k |
//! | `3.5` | ernie-3.5-128k |
//! | `speed` | ernie-speed-128k |
//! | `lite` | ernie-lite-128k |
//! | `tiny` | ernie-tiny-8k |
//!
//! ## 获取密钥
//!
//! 1. 注册百度智能云: https://cloud.baidu.com
//! 2. 进入千帆大模型平台: https://qianfan.cloud.baidu.com
//! 3. 创建应用 → 获取 API Key
//!
//! ## 免费模型
//!
//! - `ernie-speed-8k` — 免费
//! - `ernie-lite-128k` / `ernie-lite-8k` — 免费
//! - `ernie-tiny-8k` — 免费
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use nl_llm_v2::{LlmClient, PrimitiveRequest};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = LlmClient::from_preset("qianfan")?
//!         .with_api_key("YOUR-QIANFAN-API-KEY")
//!         .build();
//!
//!     let req = PrimitiveRequest::single_user_message("你好");
//!     let resp = client.complete(&req).await?;
//!     println!("{}", resp.content);
//!     Ok(())
//! }
//! ```
//!
//! ## 并发策略
//!
//! - 免费模型: 5 QPS
//! - 付费模型: 根据套餐
//! - 初始: 3

use crate::client::ClientBuilder;
use crate::model::qianfan::QianfanModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::qianfan::QianfanExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

const QIANFAN_BASE_URL: &str = "https://qianfan.baidubce.com/v2";

/// 创建百度千帆 v2 客户端构建器
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(QIANFAN_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(QianfanModelResolver::new())
        .with_extension(Arc::new(QianfanExtension::new()))
        .default_model("ernie-4.5-turbo-128k")
}
