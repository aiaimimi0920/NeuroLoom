use crate::auth::providers::qwen::QwenOAuth;
use crate::client::ClientBuilder;
use crate::model::QwenModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::qwen::QwenExtension;
use crate::site::base::qwen::QwenSite;
use std::sync::Arc;

/// 阿里云通义千问 (Qwen) OAuth API 预设
///
/// 通过 OAuth 认证访问 Qwen API，适用于 portal.qwen.ai 平台。
///
/// # 认证方式
///
/// 使用 Device Code + PKCE 授权流程：
/// 1. 自动打开浏览器进行授权
/// 2. 用户输入显示的 User Code
/// 3. 完成授权后自动获取 access_token
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("qwen_oauth")
///     .expect("Preset should exist")
///     .with_qwen_oauth()  // 触发 OAuth 登录
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello");
/// let response = client.complete(&req).await?;
/// ```
///
/// # 认证特性
///
/// - 自动打开浏览器授权页面
/// - Token 自动刷新
/// - 支持 Token 缓存到文件
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(QwenSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(QwenModelResolver::new())
        .with_extension(Arc::new(
            QwenExtension::new().with_base_url("https://portal.qwen.ai/v1"),
        ))
        .default_model("qwen-plus")
}

/// 创建带缓存的 Qwen OAuth 认证器
pub fn oauth_with_cache(cache_path: impl AsRef<std::path::Path>) -> QwenOAuth {
    QwenOAuth::new().with_cache(cache_path)
}
