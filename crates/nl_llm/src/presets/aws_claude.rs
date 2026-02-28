//! AWS Claude (Amazon Bedrock) 预设
//!
//! Amazon Bedrock 是 AWS 托管的 AI 服务平台，提供 Claude 等模型的 API 访问。
//!
//! ## 认证模式
//!
//! ### 1. API Key 模式（预设名: `aws_claude`）
//!
//! 使用 Bedrock API Key 进行认证（`Authorization: Bearer <key>`）。
//!
//! ```rust,no_run
//! let client = LlmClient::from_preset("aws_claude")
//!     .expect("preset")
//!     .with_api_key("YOUR-BEDROCK-API-KEY")
//!     .build();
//! ```
//!
//! 环境变量: `AWS_BEDROCK_API_KEY`
//!
//! ### 2. AK/SK 模式（预设名: `aws_claude_ak`）
//!
//! 使用 AWS AccessKey + SecretAccessKey 进行认证（AWS SigV4 签名）。
//!
//! ```text
//! AWS_ACCESS_KEY_ID=AKIA...
//! AWS_SECRET_ACCESS_KEY=xxxxx
//! AWS_REGION=us-east-1  (可选，默认 us-east-1)
//! ```
//!
//! > **注意**: AK/SK 模式需要请求级 SigV4 签名。
//! > 当前框架提供基础结构，实际签名逻辑需通过自定义 Authenticator 实现。
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 输入/输出价格 |
//! |---------|------|-------------|
//! | `anthropic.claude-sonnet-4-6-20250514-v1:0` | Claude Sonnet 4.6 (默认) | $3/$15 |
//! | `anthropic.claude-opus-4-6-20250514-v1:0` | Claude Opus 4.6 | $15/$75 |
//! | `anthropic.claude-sonnet-4-5-20250929-v1:0` | Claude Sonnet 4.5 | $3/$15 |
//! | `anthropic.claude-opus-4-5-20250915-v1:0` | Claude Opus 4.5 | $15/$75 |
//! | `anthropic.claude-3-5-sonnet-20241022-v2:0` | Claude 3.5 Sonnet v2 | $3/$15 |
//! | `anthropic.claude-3-5-haiku-20241022-v1:0` | Claude 3.5 Haiku | $0.8/$4 |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `aws` / `sonnet` / `claude` | Claude Sonnet 4.6 |
//! | `opus` | Claude Opus 4.6 |
//! | `haiku` | Claude 3.5 Haiku |
//!
//! ## 示例
//!
//! ```rust,no_run
//! use nl_llm::{LlmClient, PrimitiveRequest};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 使用 API Key 模式
//!     let client = LlmClient::from_preset("aws_claude")?
//!         .with_api_key("YOUR-BEDROCK-API-KEY")
//!         .build();
//!
//!     let req = PrimitiveRequest::single_user_message("你好");
//!     let resp = client.complete(&req).await?;
//!     println!("{}", resp.content);
//!     Ok(())
//! }
//! ```

use crate::client::ClientBuilder;
use crate::model::aws_claude::AwsClaudeModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::aws_claude::AwsClaudeExtension;
use crate::site::base::bedrock::BedrockSite;
use std::sync::Arc;

const DEFAULT_REGION: &str = "us-east-1";

// ============================================================
//  API Key 模式 — 预设名: "aws_claude"
// ============================================================

/// AWS Claude — API Key 模式
///
/// 使用简单的 API Key 进行认证（`Authorization: Bearer <key>`）。
///
/// ```rust,no_run
/// let client = LlmClient::from_preset("aws_claude")
///     .expect("preset")
///     .with_api_key("YOUR-BEDROCK-API-KEY")
///     .build();
/// ```
///
/// 环境变量: `AWS_BEDROCK_API_KEY`
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(BedrockSite::new(DEFAULT_REGION))
        .protocol(OpenAiProtocol {})
        .model_resolver(AwsClaudeModelResolver::new())
        .with_extension(Arc::new(AwsClaudeExtension::new()))
        .default_model("anthropic.claude-sonnet-4-6-20250514-v1:0")
}

// ============================================================
//  AK/SK 模式 — 预设名: "aws_claude_ak"
// ============================================================

/// AWS Claude — AK/SK 模式
///
/// 使用 AWS AccessKey + SecretAccessKey 进行认证（AWS SigV4 签名）。
///
/// ```text
/// AWS_ACCESS_KEY_ID=AKIA...
/// AWS_SECRET_ACCESS_KEY=xxxxx
/// AWS_REGION=us-east-1  (可选，默认 us-east-1)
/// ```
///
/// > **注意**: AK/SK 模式需要请求级 SigV4 签名。
/// > 当前框架提供基础结构，实际签名逻辑需通过自定义 Authenticator 实现。
pub fn builder_ak() -> ClientBuilder {
    // AK/SK 模式使用 Bedrock 原生 API（非 OpenAI 兼容）
    ClientBuilder::new()
        .site(BedrockSite::new(DEFAULT_REGION).with_native_api())
        .protocol(OpenAiProtocol {})
        .model_resolver(AwsClaudeModelResolver::new())
        .with_extension(Arc::new(AwsClaudeExtension::new()))
        .default_model("anthropic.claude-sonnet-4-6-20250514-v1:0")
}
