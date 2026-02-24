use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::qwen::QwenSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::QwenModelResolver;
use crate::provider::qwen::QwenExtension;

/// 阿里云通义千问 (Qwen) API 预设
///
/// 通义千问是阿里云推出的大语言模型系列，支持多种规模和能力。
///
/// ## 基本信息
///
/// - 官网：https://tongyi.aliyun.com
/// - API 端点：`https://dashscope.aliyuncs.com/compatible-mode/v1`
/// - 认证方式：Bearer Token (OAuth)
///
/// ## 基本用法
///
/// ```
/// let client = LlmClient::from_preset("qwen")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
///
/// ## 使用别名
///
/// ```
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("coder");  // 解析为 qwen3-coder-plus
/// ```
///
/// ## 支持的模型
///
/// | 模型 | 说明 |
/// |------|------|
/// | qwen3-max | 最强能力模型 |
/// | qwen3-coder-plus | 代码专精模型 |
/// | qwen-turbo | 快速响应模型 |
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(QwenSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(QwenModelResolver::new())
        .with_extension(Arc::new(QwenExtension::new()))
        .default_model("qwen3-coder-plus")
}
