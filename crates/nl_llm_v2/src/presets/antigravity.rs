use crate::client::ClientBuilder;
use crate::site::base::cloudcode::CloudCodeSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::protocol::hooks::cloudcode::CloudCodeHook;
use crate::model::antigravity::AntigravityModelResolver;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CloudCodeSite::new())
        .protocol(GeminiProtocol {})
        .protocol_hook(CloudCodeHook {})
        .model_resolver(AntigravityModelResolver::new())
        .default_model("gemini-2.5-flash")
}
