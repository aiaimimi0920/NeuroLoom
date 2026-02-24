use crate::client::ClientBuilder;
use crate::site::base::vertex_api::VertexApiSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::model::VertexModelResolver;

pub fn builder() -> ClientBuilder {
    // 注意: api_key 由 with_vertex_api_key() 注入并重建 Site。
    // 此处使用空占位符作为 fallback。
    ClientBuilder::new()
        .site(VertexApiSite::new(""))
        .protocol(GeminiProtocol {})
        .model_resolver(VertexModelResolver::new())
        .default_model("gemini-2.5-flash")
}
