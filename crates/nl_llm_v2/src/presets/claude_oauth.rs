use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::anthropic::AnthropicSite;
use crate::protocol::base::claude::ClaudeProtocol;
use crate::model::AnthropicModelResolver;
use crate::provider::anthropic::AnthropicExtension;

/// Claude OAuth 预设
///
/// 使用官方 Anthropic API，走 Bearer Token（OAuth）认证。
/// 协议：Claude 格式；模型默认：claude-sonnet-4-6
///
/// ```
/// let client = LlmClient::from_preset("claude_oauth")
///     .with_claude_oauth("~/.config/anthropic/token.json")
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
