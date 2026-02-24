use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::qwen::QwenSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::QwenModelResolver;
use crate::provider::qwen::QwenExtension;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(QwenSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(QwenModelResolver::new())
        .with_extension(Arc::new(QwenExtension::new()))
        .default_model("qwen3-coder-plus")
}
