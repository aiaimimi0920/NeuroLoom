use crate::client::ClientBuilder;
use crate::site::base::gemini::GeminiSite;
use crate::protocol::base::gemini::GeminiProtocol;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(GeminiSite::new())
        .protocol(GeminiProtocol {})
        .default_model("gemini-2.5-flash")
}
