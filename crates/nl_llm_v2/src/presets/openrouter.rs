use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://openrouter.ai/api/v1"))
        .protocol(OpenAiProtocol {})
        .default_model("anthropic/claude-3-opus")
}
