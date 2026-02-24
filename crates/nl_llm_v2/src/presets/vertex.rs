use crate::client::ClientBuilder;
use crate::site::base::vertex::VertexSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::model::VertexModelResolver;

pub fn builder() -> ClientBuilder {
    // 注意: project_id 会在 with_service_account_json() 中自动从 SA JSON 提取并重建 Site。
    // 此处使用占位符，仅在不调用 with_service_account_json() 时作为 fallback。
    ClientBuilder::new()
        .site(VertexSite::new("PLACEHOLDER_PROJECT_ID", "us-central1"))
        .protocol(GeminiProtocol {})
        .model_resolver(VertexModelResolver::new())
        .default_model("gemini-2.5-flash")
}
