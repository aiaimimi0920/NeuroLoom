use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::custom::{CustomExtension, CustomModelResolver};
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

const TOKENFLUX_BASE_URL: &str = "https://tokenflux.ai/v1";

/// TokenFlux AI Preset
///
/// TokenFlux provides a unified OpenAI-compatible API gateway.
/// Registering and using this preset simply routes to `https://tokenflux.ai/v1`.
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(TOKENFLUX_BASE_URL))
        .protocol(OpenAiProtocol {})
        // TokenFlux 为聚合网关，模型 ID 范围不固定，应走动态/透传解析。
        .model_resolver(CustomModelResolver::new())
        .with_extension(Arc::new(CustomExtension::new(TOKENFLUX_BASE_URL)))
        .default_model("gpt-4o") // standard fallback
}
