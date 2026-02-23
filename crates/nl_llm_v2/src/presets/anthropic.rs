use crate::client::ClientBuilder;
use crate::site::base::anthropic::AnthropicSite;
use crate::protocol::base::claude::ClaudeProtocol;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(AnthropicSite::new())
        .protocol(ClaudeProtocol {})
        .default_model("claude-sonnet-4-20250514")
}
