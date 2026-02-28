use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::model::ollama::OllamaModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::ollama::OllamaExtension;
use crate::site::base::openai::OpenAiSite;

const OLLAMA_OPENAI_BASE_URL: &str = "http://127.0.0.1:11434/v1";
const OLLAMA_DEFAULT_MODEL: &str = "llama3";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(OLLAMA_OPENAI_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(OllamaModelResolver::new())
        .with_extension(Arc::new(OllamaExtension::new()))
        .default_model(OLLAMA_DEFAULT_MODEL)
}
