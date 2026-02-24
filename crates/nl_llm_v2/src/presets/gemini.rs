use crate::client::ClientBuilder;
use crate::site::base::gemini::GeminiSite;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::model::GeminiModelResolver;

/// Google Gemini 官方 API 预设
///
/// Google Gemini 是 Google 的大语言模型系列，支持多模态和长上下文。
///
/// ## 基本信息
///
/// - 官网：https://ai.google.dev
/// - API 端点：`https://generativelanguage.googleapis.com/v1beta`
/// - 认证方式：API Key（通过 URL query `?key=` 传递）
///
/// ## 基本用法
///
/// ```
/// let client = LlmClient::from_preset("gemini")
///     .expect("Preset should exist")
///     .with_gemini_api_key("your-api-key")
///     .build();
/// ```
///
/// ## 注意
///
/// Gemini API Key 不走 HTTP Header，而是拼在 URL 中。
/// 必须使用 `with_gemini_api_key()` 方法注入 Key。
pub fn builder() -> ClientBuilder {
    // 注意: GeminiExtension 需要 API Key，在 with_gemini_api_key() 时注入。
    // 此处仅设置 Site / Protocol / ModelResolver 基础配置。
    ClientBuilder::new()
        .site(GeminiSite::new())
        .protocol(GeminiProtocol {})
        .model_resolver(GeminiModelResolver::new())
        .default_model("gemini-2.5-flash")
}
