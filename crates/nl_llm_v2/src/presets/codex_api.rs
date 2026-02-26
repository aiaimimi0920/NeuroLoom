use crate::client::ClientBuilder;
use crate::model::CodexModelResolver;
use crate::protocol::base::codex::CodexProtocol;
use crate::provider::codex::CodexExtension;
use crate::site::base::codex_api::CodexApiSite;
use std::sync::Arc;

/// Codex API 预设（API Key 模式）
///
/// 通过 OpenAI 官方 API Key 访问 Codex 模型。
/// 站点: `api.openai.com/v1/responses`
/// 协议: CodexProtocol（Responses API: input + instructions）
/// 认证: 标准 Bearer API Key
///
/// ```
/// let client = LlmClient::from_preset("codex_api")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CodexApiSite::new())
        .protocol(CodexProtocol {})
        .model_resolver(CodexModelResolver::new())
        .with_extension(Arc::new(CodexExtension::new()))
        .default_model("gpt-5.1-codex")
}
