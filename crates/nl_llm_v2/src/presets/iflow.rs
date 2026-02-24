use crate::client::ClientBuilder;
use crate::site::base::iflow::IFlowSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::protocol::hooks::iflow::IflowThinkingHook;
use crate::model::iflow::IFlowModelResolver;
use crate::provider::iflow::IFlowExtension;
use std::sync::Arc;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(IFlowSite::new())
        .protocol(OpenAiProtocol {})
        .with_protocol_hook(Arc::new(IflowThinkingHook {}))
        .with_extension(Arc::new(IFlowExtension {}))
        .model_resolver(IFlowModelResolver::new())
        .default_model("qwen3-max")
}
