use crate::client::ClientBuilder;
use crate::model::VertexModelResolver;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::provider::vertex;
use crate::site::base::vertex::VertexSite;

/// Google Vertex AI API 预设
///
/// Vertex AI 是 Google Cloud 的 AI 平台，提供 Gemini 等模型的托管服务。
///
/// ## 基本信息
///
/// - 官网：https://cloud.google.com/vertex-ai
/// - API 端点：`https://{region}-aiplatform.googleapis.com/v1`
/// - 认证方式：Service Account JSON 或 API Key
///
/// ## 使用 Service Account 认证
///
/// ```
/// let client = LlmClient::from_preset("vertex")
///     .expect("Preset should exist")
///     .with_service_account_json(r#"{"type": "service_account", ...}"#)
///     .build();
/// ```
///
/// ## 使用 API Key 认证
///
/// ```
/// let client = LlmClient::from_preset("vertex_api")
///     .expect("Preset should exist")
///     .with_vertex_api_key("AIza...")
///     .build();
/// ```
///
/// ## 注意
///
/// - Service Account 认证需要 `vertex` 预设
/// - API Key 认证需要 `vertex_api` 预设
/// - project_id 会自动从 Service Account JSON 中提取
pub fn builder() -> ClientBuilder {
    // 注意: project_id 会在 with_service_account_json() 中自动从 SA JSON 提取并重建 Site。
    // 此处使用占位符，仅在不调用 with_service_account_json() 时作为 fallback。
    ClientBuilder::new()
        .site(VertexSite::new("PLACEHOLDER_PROJECT_ID", "us-central1"))
        .protocol(GeminiProtocol {})
        .model_resolver(VertexModelResolver::new())
        .with_extension(vertex::extension("PLACEHOLDER_PROJECT_ID", "us-central1"))
        .default_model("gemini-2.5-flash")
}
