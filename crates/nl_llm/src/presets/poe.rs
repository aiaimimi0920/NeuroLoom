use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::poe::{PoeExtension, PoeModelResolver};
use crate::site::base::openai::OpenAiSite;

pub fn builder() -> ClientBuilder {
    let base_url =
        std::env::var("POE_BASE_URL").unwrap_or_else(|_| "https://api.poe.com/v1".to_string());

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(PoeModelResolver::new())
        .with_extension(Arc::new(PoeExtension::new(base_url)))
        .default_model("gpt-4o-mini")
}
