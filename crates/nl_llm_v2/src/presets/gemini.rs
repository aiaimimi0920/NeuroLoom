use crate::client::ClientBuilder;
use crate::site::base::gemini::GeminiSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::model::GeminiModelResolver;

pub fn builder() -> ClientBuilder {
    // 注意: GeminiExtension 需要 API Key，在 with_gemini_api_key() 时注入。
    // 此处仅设置 Site / Protocol / ModelResolver 基础配置。
    ClientBuilder::new()
        .site(GeminiSite::new())
        .protocol(GeminiProtocol {})
        .model_resolver(GeminiModelResolver::new())
        .default_model("gemini-2.5-flash")
}
