use crate::client::ClientBuilder;
use crate::provider::moka::MokaModelResolver;
use crate::site::base::moka::MokaSite;
use crate::auth::providers::ApiKeyAuth;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::openai::OpenAiExtension;
use std::sync::Arc;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(MokaSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(MokaModelResolver::new())
        // Include the OpenAI extension to optionally pull models
        .with_extension(Arc::new(OpenAiExtension::new()))
}
