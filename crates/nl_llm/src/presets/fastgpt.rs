use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::fastgpt::FastGptExtension;
use crate::site::base::fastgpt::FastGptSite;

pub fn default_builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(FastGptSite::new())
        .protocol(OpenAiProtocol {})
        .with_extension(Arc::new(FastGptExtension::new()))
        .default_model("fastgpt-default")
}
