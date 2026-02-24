use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::openrouter::OpenRouterModelResolver;
use crate::provider::openrouter;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://openrouter.ai/api/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(OpenRouterModelResolver::new())
        .with_extension(openrouter::extension())
        .default_model("anthropic/claude-3-opus")
}
