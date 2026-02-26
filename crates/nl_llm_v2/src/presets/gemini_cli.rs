use crate::client::ClientBuilder;
use crate::model::GeminiCliModelResolver;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::protocol::hooks::cloudcode::CloudCodeHook;
use crate::provider::gemini_cli::GeminiCliExtension;
use crate::site::base::cloudcode::CloudCodeSite;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CloudCodeSite::new())
        .protocol(GeminiProtocol {})
        .with_protocol_hook(std::sync::Arc::new(CloudCodeHook {}))
        .with_extension(std::sync::Arc::new(GeminiCliExtension {}))
        .model_resolver(GeminiCliModelResolver::new())
        .default_model("gemini-2.5-flash")
}
