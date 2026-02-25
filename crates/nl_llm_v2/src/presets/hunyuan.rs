use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::provider::hunyuan::{HunyuanHook, HunyuanModelResolver};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::hunyuan::HunyuanSite;

pub fn builder() -> ClientBuilder {
    let hook = Arc::new(HunyuanHook {});
    ClientBuilder::new()
        .site(HunyuanSite::new())
        .protocol(OpenAiProtocol {})
        .with_extension(hook.clone())
        .with_protocol_hook(hook)
        .model_resolver(HunyuanModelResolver::new())
        .default_model("hunyuan-turbos-latest")
}
