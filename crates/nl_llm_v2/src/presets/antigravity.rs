use crate::client::ClientBuilder;
use crate::site::base::cloudcode::CloudCodeSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::protocol::hooks::cloudcode::CloudCodeHook;
use crate::model::antigravity::AntigravityModelResolver;
use crate::provider::antigravity::AntigravityExtension;
use std::sync::Arc;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CloudCodeSite::new())
        .protocol(GeminiProtocol {})
        .with_protocol_hook(Arc::new(CloudCodeHook {}))
        .with_extension(Arc::new(AntigravityExtension {}))
        .model_resolver(AntigravityModelResolver::new())
        .default_model("gemini-2.5-flash")
}
