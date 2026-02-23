use crate::client::ClientBuilder;
use crate::site::base::vertex::VertexSite;
use crate::protocol::base::gemini::GeminiProtocol;

pub fn builder() -> ClientBuilder {
    // 占位符，用户应该通过 .site() 覆盖或 .with_service_account_json() 设置
    ClientBuilder::new()
        .site(VertexSite::new("PLACEHOLDER_PROJECT_ID", "us-central1"))
        .protocol(GeminiProtocol {})
        .default_model("gemini-2.5-flash")
}
