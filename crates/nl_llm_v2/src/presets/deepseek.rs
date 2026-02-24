use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::DeepSeekModelResolver;
use crate::provider::deepseek::DeepSeekExtension;

/// DeepSeek API 预设
///
/// DeepSeek 是一家中国 AI 公司，提供高性价比的 LLM API 服务。
/// 使用 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.deepseek.com/v1`
/// - **认证**: `Authorization: Bearer <DEEPSEEK_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **余额查询**: 支持
///
/// # 模型说明
///
/// | 模型 | 说明 | 能力 |
/// |------|------|------|
/// | `deepseek-chat` | 通用对话模型 | Chat, Tools, Streaming |
/// | `deepseek-reasoner` | 深度推理模型 | Chat, Streaming, Thinking |
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `deepseek` / `ds` | `deepseek-chat` | 对话模型 |
/// | `reasoner` / `r1` / `think` | `deepseek-reasoner` | 推理模型 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("deepseek")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .with_concurrency()  // 可选：启用并发控制
///     .build();
///
/// // 使用别名
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("ds");  // 自动解析为 deepseek-chat
///
/// // 使用推理模型
/// let req = PrimitiveRequest::single_user_message("Solve this problem")
///     .with_model("think");  // 自动解析为 deepseek-reasoner
/// ```
///
/// # 余额查询
///
/// ```rust,no_run
/// let balance = client.get_balance().await?;
/// println!("余额: {:?}", balance);
/// ```
const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(DEEPSEEK_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(DeepSeekModelResolver::new())
        .with_extension(Arc::new(DeepSeekExtension::new().with_base_url(DEEPSEEK_BASE_URL)))
        .default_model("deepseek-chat")
}
