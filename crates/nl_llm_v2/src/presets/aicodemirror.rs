use crate::client::ClientBuilder;
use crate::model::aicodemirror::AiCodeMirrorModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::aicodemirror::AiCodeMirrorExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// AICodeMirror AI 编程代理平台预设
///
/// AICodeMirror 提供 Claude Code、Codex、Gemini 的统一代理服务，
/// 支持全球高保线路和国内优化线路。
///
/// # 平台特性
///
/// - **默认网关**: `https://api.aicodemirror.com/api/claudecode/v1`（ClaudeCode 通道）
/// - **认证**: `Authorization: Bearer <key>`，密钥格式 `sk-ant-api03-...`
/// - **协议**: OpenAI 兼容（ClaudeCode 通道同时支持 Anthropic 原生）
/// - **并发**: 官方上限 20，初始并发 5
///
/// # 多线路端点
///
/// ## 全球高保线路
///
/// | 通道 | 端点 |
/// |------|------|
/// | ClaudeCode | `https://api.aicodemirror.com/api/claudecode/v1` |
/// | Codex | `https://api.aicodemirror.com/api/codex/backend-api/codex/v1` |
/// | Gemini | `https://api.aicodemirror.com/api/gemini/v1` |
///
/// ## 国内优化线路
///
/// | 通道 | 端点 |
/// |------|------|
/// | ClaudeCode | `https://api.claudecode.net.cn/api/claudecode/v1` |
/// | Codex | `https://api.claudecode.net.cn/api/codex/backend-api/codex/v1` |
/// | Gemini | `https://api.claudecode.net.cn/api/gemini/v1` |
///
/// # 支持的模型
///
/// | 模型 ID | 说明 | 上下文长度 |
/// |---------|------|-----------|
/// | `claude-sonnet-4-6` | Claude 4.6 Sonnet 最新平衡模型 | 200K |
/// | `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5（默认）| 200K |
/// | `claude-haiku-4-5-20251001` | Claude 4.5 Haiku 快速高效 | 200K |
/// | `claude-opus-4-20250514` | Claude 4 Opus 旗舰模型 | 200K |
/// | `claude-sonnet-4-20250514` | Claude 4 Sonnet | 200K |
/// | `claude-3-7-sonnet-20250219` | Claude 3.7 Sonnet 扩展思考 | 200K |
/// | `claude-3-5-sonnet-20241022` | Claude 3.5 Sonnet | 200K |
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `aicodemirror` / `sonnet` / `claude` | `claude-sonnet-4-5-20250929` | 默认模型 |
/// | `sonnet-4.6` | `claude-sonnet-4-6` | 最新 4.6 版本 |
/// | `opus` | `claude-opus-4-20250514` | 旗舰模型 |
/// | `haiku` | `claude-haiku-4-5-20251001` | 快速模型 |
/// | `3.7` | `claude-3-7-sonnet-20250219` | 3.7 扩展思考 |
/// | `3.5` | `claude-3-5-sonnet-20241022` | 3.5 经典版本 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// // 使用默认 ClaudeCode 通道
/// let client = LlmClient::from_preset("aicodemirror")
///     .expect("Preset should exist")
///     .with_api_key("sk-ant-api03-xxx")
///     .build();
///
/// // 使用模型别名
/// let req = PrimitiveRequest::single_user_message("你好!")
///     .with_model("sonnet-4.6");  // 最新 4.6 版本
///
/// // 切换到国内线路
/// let client = LlmClient::from_preset("aicodemirror")
///     .expect("Preset should exist")
///     .with_api_key("sk-ant-api03-xxx")
///     .with_base_url("https://api.claudecode.net.cn/api/claudecode/v1")
///     .build();
/// ```
const AICODEMIRROR_BASE_URL: &str = "https://api.aicodemirror.com/api/claudecode/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(AICODEMIRROR_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(AiCodeMirrorModelResolver::new())
        .with_extension(Arc::new(
            AiCodeMirrorExtension::new().with_base_url(AICODEMIRROR_BASE_URL),
        ))
        .default_model("claude-sonnet-4-5-20250929")
}
