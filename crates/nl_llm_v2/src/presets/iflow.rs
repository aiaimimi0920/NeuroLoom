use crate::client::ClientBuilder;
use crate::site::base::iflow::IFlowSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::protocol::hooks::iflow::IflowThinkingHook;
use crate::model::iflow::IFlowModelResolver;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(IFlowSite::new())
        .protocol(OpenAiProtocol {})
        .protocol_hook(IflowThinkingHook {})
        .model_resolver(IFlowModelResolver::new())
        .default_model("qwen3-max")
}
