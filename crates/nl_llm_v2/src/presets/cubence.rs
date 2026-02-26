use crate::client::ClientBuilder;
use crate::model::cubence::CubenceModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::cubence::CubenceExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// Cubence AI API Gateway 预设
///
/// Cubence 是专业 AI 工具代理平台，支持 Claude Code、Codex、Gemini CLI 等。
///
/// # 平台特性
///
/// - **网关**: `https://api.cubence.com/v1`
/// - **认证**: Bearer，密钥 `sk-user-` 前缀
/// - **协议**: OpenAI 兼容
/// - **类型**: API 聚合平台
///
/// # 可用端点
///
/// | 端点 | 说明 |
/// |------|------|
/// | `https://api.cubence.com` | 默认推荐 |
/// | `https://api-dmit.cubence.com` | 备用线路 |
/// | `https://api-bwg.cubence.com` | 备用线路 |
/// | `https://api-cf.cubence.com` | Cloudflare 线路 |
///
/// # 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `claude-sonnet-4-5-20250929` | 200K | Claude Sonnet 4.5 |
/// | `claude-3-5-sonnet-20241022` | 200K | Claude 3.5 Sonnet |
/// | `gpt-4o` | 128K | GPT-4o |
/// | `gpt-4o-mini` | 128K | GPT-4o Mini |
/// | `gemini-2.0-flash` | 1M | Gemini 2.0 Flash |
/// | `gemini-2.5-pro` | 1M | Gemini 2.5 Pro |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("cubence")
///     .expect("Preset should exist")
///     .with_api_key("sk-user-xxx")
///     .build();
/// ```
const CUBENCE_BASE_URL: &str = "https://api.cubence.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(CUBENCE_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(CubenceModelResolver::new())
        .with_extension(Arc::new(
            CubenceExtension::new().with_base_url(CUBENCE_BASE_URL),
        ))
        .default_model("claude-sonnet-4-5-20250929")
}
