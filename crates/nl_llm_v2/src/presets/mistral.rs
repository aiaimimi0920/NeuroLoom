use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::mistral::{MistralExtension, MistralHook, MistralModelResolver};
use crate::site::base::mistral::MistralSite;

/// 创建 Mistral 预设
/// 组装 MistralSite、OpenAiProtocol、MistralHook 和 MistralModelResolver。
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(MistralSite::new())
        .protocol(OpenAiProtocol {})
        .with_protocol_hook(std::sync::Arc::new(MistralHook::new()))
        .model_resolver(MistralModelResolver::new())
        .with_extension(Arc::new(MistralExtension::new()))
        .default_model("mistral-large-latest")
}
