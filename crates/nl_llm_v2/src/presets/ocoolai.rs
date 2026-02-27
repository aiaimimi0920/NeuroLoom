use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::ocoolai::{OcoolAiExtension, OcoolAiModelResolver};
use crate::site::base::openai::OpenAiSite;

pub fn builder() -> ClientBuilder {
    let base_url = std::env::var("OCOOLAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.ocoolai.com/v1".to_string());

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(OcoolAiModelResolver::new())
        .with_extension(Arc::new(OcoolAiExtension::new(base_url)))
        .default_model("gpt-4o-mini")
}
