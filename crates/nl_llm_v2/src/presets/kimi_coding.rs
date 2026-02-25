use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::KimiModelResolver;
use crate::provider::kimi::KimiExtension;

/// 月之暗面 Kimi For Coding (专门构建版) 预设
///
/// 独立切分于标准 `kimi` 的高定制预设。
///
/// 专精于代码解析与编程辅助任务，直接路由至 `https://api.kimi.com/v1` 域名，并锁定 `kimi-for-coding` 模型。
/// 这能让意图明确的服务（例如编程助手）获得最高权重的表现。
const KIMI_CODING_BASE_URL: &str = "https://api.kimi.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        // 覆盖成编程特定的 Endpoint
        .site(OpenAiSite::new().with_base_url(KIMI_CODING_BASE_URL))
        .protocol(OpenAiProtocol {})
        // 依然复用统一解析体系
        .model_resolver(KimiModelResolver::new())
        // 确保各种 /balance 的附加路由走专门 Endpoint
        .with_extension(Arc::new(KimiExtension::new().with_base_url(KIMI_CODING_BASE_URL)))
        // 默认将该通道指派为写代码最棒的模型
        .default_model("kimi-for-coding")
}
