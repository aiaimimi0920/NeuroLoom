use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::KimiModelResolver;
use crate::provider::kimi::KimiExtension;
use crate::auth::providers::kimi::KimiOAuth;

/// 月之暗面 Kimi (Moonshot) OAuth 预设
///
/// 通过 Kimi 网页版的 Device Code 流进行 Oauth 认证，调用专属 Kimi Web 的节点。
/// 适用于希望无需 API Key 即可白嫖使用 Kimi 服务（包含其强大的 K2.5 模型）的用户。
///
/// # 认证方式
///
/// 使用 RFC 8628 Device Authorization Grant 流程：
/// 1. 自动弹出浏览器进行授权 / 或者打印地址让用户手机扫码
/// 2. 输入显示的 User Code
/// 3. 系统后台轮询 Token，拿到后持久化并用于所有后续 API 调用
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("kimi_oauth")
///     .expect("Preset should exist")
///     // 触发或直接读取之前的 OAuth 缓存信息
///     .with_kimi_oauth() 
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        // Kimi Oauth 的默认通讯节点
        .site(OpenAiSite::new().with_base_url("https://api.kimi.com/coding"))
        .protocol(OpenAiProtocol {})
        .model_resolver(KimiModelResolver::new())
        .with_extension(Arc::new(KimiExtension::new().with_base_url("https://api.kimi.com/coding")))
        .default_model("kimi-k2.5")
}

/// 快捷带持久化缓存路径的构造辅助
pub fn oauth_with_cache(cache_path: impl AsRef<std::path::Path>) -> KimiOAuth {
    KimiOAuth::new(cache_path)
}
