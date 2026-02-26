use crate::LlmClientBuilder;
use crate::client::LlmClient;
use crate::protocol::openai::OpenAiProtocol;
use crate::site::base::fastgpt::FastGptSite;
use crate::provider::fastgpt;
use std::sync::Arc;

pub fn default_builder() -> LlmClientBuilder {
    LlmClient::builder()
        .with_site(Arc::new(FastGptSite::new()))
        .with_protocol(Arc::new(OpenAiProtocol::new("fastgpt")))
        .with_extension(fastgpt::extension())
}
