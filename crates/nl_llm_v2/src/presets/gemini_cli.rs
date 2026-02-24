use crate::client::ClientBuilder;
use crate::site::base::cloudcode::CloudCodeSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::protocol::hooks::cloudcode::CloudCodeHook;
use crate::provider::gemini_cli::GeminiCliExtension;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CloudCodeSite::new())
        .protocol(GeminiProtocol {})
        .with_protocol_hook(std::sync::Arc::new(CloudCodeHook {}))
        .with_extension(std::sync::Arc::new(GeminiCliExtension {}))
        .default_model("gemini-2.5-flash")
}
