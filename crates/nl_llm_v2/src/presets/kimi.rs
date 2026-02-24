use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::kimi::KimiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::KimiModelResolver;
use crate::provider::kimi::KimiExtension;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(KimiSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(KimiModelResolver::new())
        .with_extension(Arc::new(KimiExtension::new()))
        .default_model("k2")
}
