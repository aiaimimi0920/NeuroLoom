use crate::client::ClientBuilder;
use crate::site::base::iflow::IFlowSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::protocol::hooks::iflow::IflowThinkingHook;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(IFlowSite::new())
        .protocol(OpenAiProtocol {})
        .protocol_hook(IflowThinkingHook {})
        .default_model("qwen3-max")
}
