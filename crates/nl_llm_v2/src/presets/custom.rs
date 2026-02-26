use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::custom::CustomModelResolver;
use crate::provider::openai::OpenAiExtension;
use crate::site::base::openai::OpenAiSite;

use std::sync::Arc;

/// 默认的占位 Base URL，若未设置环境变量则使用此 URL 回落。
const CUSTOM_BASE_URL_PLACEHOLDER: &str = "https://api.openai.com/v1";

/// “自定义渠道”构建器
pub fn builder() -> ClientBuilder {
    // 允许通过 CUSTOM_BASE_URL 独立覆盖自定义渠道的代理地址
    let base_url = std::env::var("CUSTOM_BASE_URL")
        .unwrap_or_else(|_| CUSTOM_BASE_URL_PLACEHOLDER.to_string());

    ClientBuilder::new()
        // 复用标准的 OpenAiSite 作为发包宿主，支持 /chat/completions 路由
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        // 动态传递、兼容任何命名的模型解析器
        .model_resolver(CustomModelResolver::new())
        .with_extension(Arc::new(OpenAiExtension::new()))
}
