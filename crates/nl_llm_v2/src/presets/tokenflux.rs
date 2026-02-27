use crate::auth::providers::ApiKeyAuth;
use crate::client::ClientBuilder;
use crate::model::DefaultModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// TokenFlux AI Preset
///
/// TokenFlux provides a unified OpenAI-compatible API gateway.
/// Registering and using this preset simply routes to `https://tokenflux.ai/v1`.
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://tokenflux.ai/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(DefaultModelResolver::new()) // standard models or tokenflux specific models
        .default_model("gpt-4o") // standard fallback
}
