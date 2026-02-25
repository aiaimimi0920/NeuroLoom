//! Cohere 预设
//!
//! Cohere 提供 Command 系列模型，支持文本生成、对话、翻译和推理。
//!
//! ## API 端点
//!
//! - **原生 API**: `https://api.cohere.com/v2/chat`
//! - **OpenAI 兼容**: `https://api.cohere.com/compatibility/v1/chat/completions`
//! - **认证**: `Authorization: Bearer <key>`
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文长度 |
//! |---------|------|-----------|
//! | `command-a-03-2025` | Command A — 最新旗舰模型（默认） | 256K |
//! | `command-a-vision-07-2025` | Command A Vision — 支持图像输入 | 256K |
//! | `command-a-reasoning-08-2025` | Command A Reasoning — 推理增强 | 256K |
//! | `command-a-translate-08-2025` | Command A Translate — 翻译专用 | 256K |
//! | `command-r-plus-08-2024` | Command R+ — 强力模型 | 128K |
//! | `command-r-08-2024` | Command R — 平衡模型 | 128K |
//! | `command-r7b-12-2024` | Command R 7B — 轻量快速 | 128K |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `cohere` / `command` / `command-a` | command-a-03-2025 |
//! | `vision` | command-a-vision-07-2025 |
//! | `reasoning` | command-a-reasoning-08-2025 |
//! | `translate` | command-a-translate-08-2025 |
//! | `r+` | command-r-plus-08-2024 |
//! | `r` | command-r-08-2024 |
//! | `r7b` | command-r7b-12-2024 |
//!
//! ## 密钥类型
//!
//! - **生产密钥**: 付费使用，无速率限制
//! - **试用密钥**: 免费，有速率限制（20 RPM）
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use nl_llm_v2::{LlmClient, PrimitiveRequest};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = LlmClient::from_preset("cohere")?
//!         .with_api_key("YOUR-COHERE-API-KEY")
//!         .build();
//!
//!     let req = PrimitiveRequest::single_user_message("你好");
//!     let resp = client.complete(&req).await?;
//!     println!("{}", resp.content);
//!     Ok(())
//! }
//! ```

use crate::client::ClientBuilder;
use crate::model::cohere::CohereModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::cohere::CohereExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

const COHERE_BASE_URL: &str = "https://api.cohere.com/compatibility/v1";

/// 创建 Cohere 客户端构建器
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(COHERE_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(CohereModelResolver::new())
        .with_extension(Arc::new(CohereExtension::new()))
        .default_model("command-a-03-2025")
}
