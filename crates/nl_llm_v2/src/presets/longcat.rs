use crate::client::ClientBuilder;
use crate::model::longcat::LongcatModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::longcat::LongcatExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// Longcat AI 预设
///
/// 通过标准 OpenAI 兼容格式连接 Longcat 服务。
///
/// # 平台特性
///
/// - **网关**: `https://api.longcat.chat/openai/v1`
/// - **认证**: `Authorization: Bearer <API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 模型说明
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `LongCat-Flash-Chat` | Chat, Tools, Streaming | 128K | 基础模型 |
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `longcat` / `flash` | `LongCat-Flash-Chat` | 默认模型 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("longcat")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .with_concurrency()  // 可选：启用并发控制
///     .build();
///
/// // 使用别名
/// let req = PrimitiveRequest::single_user_message("你好")
///     .with_model("flash");  // 自动解析为 LongCat-Flash-Chat
/// ```
const LONGCAT_BASE_URL: &str = "https://api.longcat.chat/openai/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(LONGCAT_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(LongcatModelResolver::new())
        .with_extension(Arc::new(LongcatExtension::new().with_base_url(LONGCAT_BASE_URL)))
        .default_model("LongCat-Flash-Chat")
}
