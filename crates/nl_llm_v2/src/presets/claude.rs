use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::anthropic::AnthropicSite;
use crate::protocol::base::claude::ClaudeProtocol;
use crate::model::AnthropicModelResolver;
use crate::provider::anthropic::AnthropicExtension;

/// Claude API Key 预设
///
/// 使用官方 Anthropic API，走 x-api-key header 认证。
/// 协议：Claude 格式；模型默认：claude-sonnet-4-6
///
/// ```
/// let client = LlmClient::from_preset("claude")
///     .with_claude_api_key("sk-ant-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(AnthropicSite::new())
        .protocol(ClaudeProtocol {})
        .model_resolver(AnthropicModelResolver::new())
        .with_extension(Arc::new(AnthropicExtension::new()))
        .default_model("claude-sonnet-4-6")
}
