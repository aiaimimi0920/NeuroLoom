use crate::client::ClientBuilder;
use crate::model::rightcode::RightCodeModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::rightcode::RightCodeExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// RightCode 企业级 AI Agent 中转平台预设
///
/// Right Code 涵盖 Claude Code、Codex、Gemini CLI、Grok Code 的统一接入与管理。
///
/// # 平台特性
///
/// - **网关**: `https://www.right.codes/codex/v1`
/// - **认证**: Bearer，密钥 `sk-` 前缀
/// - **协议**: OpenAI 兼容
/// - **类型**: 企业级 AI Agent 中转平台
///
/// # 可用端点
///
/// | 端点 | 说明 |
/// |------|------|
/// | `/codex/v1` | Codex OpenAI 兼容通道 |
/// | `/claude/v1` | Anthropic 原生通道 |
/// | `/gemini/v1beta` | Google 原生通道 |
///
/// # 支持的模型
///
/// | 模型 ID | 输入价格 | 输出价格 |
/// |---------|---------|---------|
/// | `gpt-5` | $1.25/M | $10.00/M |
/// | `gpt-5-codex` | $1.25/M | $10.00/M |
/// | `gpt-5-codex-mini` | $0.25/M | $2.00/M |
/// | `gpt-5.1` | $1.25/M | $10.00/M |
/// | `gpt-5.1-codex` | $1.25/M | $10.00/M |
/// | `gpt-5.1-codex-max` | $1.25/M | $10.00/M |
/// | `gpt-5.1-codex-mini` | $0.25/M | $2.00/M |
/// | `gpt-5.2` | $1.75/M | $14.00/M |
/// | `gpt-5.2-codex` | $1.75/M | $14.00/M |
/// | `gpt-5.2-high/medium/low/xhigh` | $1.75/M | $14.00/M |
/// | `gpt-5.3-codex` | $1.75/M | $14.00/M |
/// | `gpt-5.3-codex-high/medium/low/xhigh` | $1.75/M | $14.00/M |
///
/// > **注意**: 用户套餐决定可用模型范围，Codex 套餐仅支持 codex 系列
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("rightcode")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
const RIGHTCODE_BASE_URL: &str = "https://www.right.codes/codex/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(RIGHTCODE_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(RightCodeModelResolver::new())
        .with_extension(Arc::new(RightCodeExtension::new().with_base_url(RIGHTCODE_BASE_URL)))
        .default_model("gpt-5.1-codex-mini")
}
