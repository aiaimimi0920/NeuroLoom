use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::kimi::KimiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::KimiModelResolver;
use crate::provider::kimi::KimiExtension;

/// Moonshot Kimi (月之暗面) API 预设
///
/// Kimi 是月之暗面推出的 AI 助手，支持长上下文和文件理解。
///
/// ## 基本信息
///
/// - 官网：https://kimi.moonshot.cn
/// - API 端点：`https://api.moonshot.cn/v1`
/// - 认证方式：Bearer Token
///
/// ## 基本用法
///
/// ```
/// let client = LlmClient::from_preset("kimi")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
///
/// ## 使用别名
///
/// ```
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("kimi");  // 解析为 k2
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(KimiSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(KimiModelResolver::new())
        .with_extension(Arc::new(KimiExtension::new()))
        .default_model("k2")
}
