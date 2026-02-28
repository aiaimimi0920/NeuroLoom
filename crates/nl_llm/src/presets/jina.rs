use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::jina::JinaModelResolver;
use crate::site::base::jina::JinaSite;

/// 创建 Jina 预设
/// 组装 JinaSite、OpenAiProtocol (复用 OpenAI JSON 协议) 和 JinaModelResolver。
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(JinaSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(JinaModelResolver::new())
}
