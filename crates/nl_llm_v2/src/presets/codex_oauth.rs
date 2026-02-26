use crate::client::ClientBuilder;
use crate::model::CodexModelResolver;
use crate::protocol::base::codex::CodexProtocol;
use crate::provider::codex::CodexExtension;
use crate::site::base::codex::CodexSite;
use std::sync::Arc;

/// Codex OAuth 预设
///
/// 使用 OpenAI Codex API，走 OAuth Bearer Token 认证。
/// 协议：CodexProtocol（Responses API: input + instructions 格式）
/// 模型默认：gpt-5.1-codex
///
/// ```
/// let client = LlmClient::from_preset("codex_oauth")
///     .with_codex_oauth("~/.config/codex/token.json")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CodexSite::new())
        .protocol(CodexProtocol {})
        .model_resolver(CodexModelResolver::new())
        .with_extension(Arc::new(CodexExtension::new()))
        .default_model("gpt-5.1-codex")
}
