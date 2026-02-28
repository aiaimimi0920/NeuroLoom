use crate::client::ClientBuilder;
use crate::model::amp::AmpModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::amp::AmpExtension;
use crate::site::base::amp::{AmpConfig, AmpSite};
use std::sync::Arc;

/// Sourcegraph Amp 预设 (ampcode.com)
///
/// Sourcegraph Amp 是一个 AI 编码助手平台，提供 OpenAI 兼容的供应商路由接口。
/// 通过 `/api/provider/{provider}/v1/chat/completions` 路径模式
/// 将请求路由到不同的后端供应商（OpenAI / Anthropic / Google 等）。
///
/// # 平台特性
///
/// - **端点**: `https://ampcode.com`
/// - **认证**: `Authorization: Bearer <AMP_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **模型**: 聚合 GPT、Claude、Gemini 等多平台模型
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("amp")
///     .expect("Preset should exist")
///     .with_api_key("your-amp-api-key")
///     .build();
///
/// // 使用模型别名
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("best");  // 自动解析为 gemini-2.5-pro
/// ```
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `best` | `gemini-2.5-pro` | 最强能力 |
/// | `fast` | `gemini-2.5-flash` | 快速响应 |
/// | `cheap` | `gpt-4o-mini` | 低成本 |
/// | `claude` | `claude-sonnet-4-20250514` | Claude Sonnet |
/// | `reasoning` | `o1` | 推理模型 |
///
/// # 配置共享
///
/// AmpSite 和 AmpExtension 共享同一份 `AmpConfig`（通过 Arc），
/// 确保 base_url 和 provider 修改后两者保持一致。
pub fn builder() -> ClientBuilder {
    // 共享配置：确保 Site 和 Extension 使用同一份 base_url/provider
    let config = Arc::new(AmpConfig::new());

    ClientBuilder::new()
        .site(AmpSite::from_config(config.clone()))
        .protocol(OpenAiProtocol {})
        .model_resolver(AmpModelResolver::new())
        .with_extension(Arc::new(AmpExtension::from_config(config)))
        .default_model("gpt-4o") // 默认使用 GPT-4o
}
