use crate::auth::providers::OllamaAuth;
use crate::client::ClientBuilder;
use crate::model::OpenAiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

const LMSTUDIO_BASE_URL: &str = "http://127.0.0.1:1234/v1";
const LMSTUDIO_DEFAULT_MODEL: &str = "local-model";

/// LM Studio (OpenAI-compatible local server) preset.
///
/// LM Studio 默认启动在 `http://127.0.0.1:1234` 并暴露 OpenAI 兼容接口。
/// 该预设默认使用空鉴权（本地可直接调用），如需鉴权可链式调用
/// `.with_ollama_auth("your-key")` 覆盖。
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(LMSTUDIO_BASE_URL))
        .protocol(OpenAiProtocol {})
        .auth(OllamaAuth::new(""))
        .model_resolver(OpenAiModelResolver::new())
        .default_model(LMSTUDIO_DEFAULT_MODEL)
}
