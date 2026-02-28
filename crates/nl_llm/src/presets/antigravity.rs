use crate::client::ClientBuilder;
use crate::model::antigravity::AntigravityModelResolver;
use crate::protocol::base::gemini::GeminiProtocol;
use crate::protocol::hooks::cloudcode::CloudCodeHook;
use crate::provider::antigravity::AntigravityExtension;
use crate::site::base::cloudcode::CloudCodeSite;
use std::sync::Arc;

/// Antigravity (CloudCode PA) API 预设
///
/// Antigravity 是 Google 内部的 CloudCode PA 平台，提供 Gemini 和 Claude 模型访问。
///
/// ## 基本信息
///
/// - 认证方式：OAuth (Google 账户)
/// - 协议：Gemini 格式（Claude 模型通过翻译层支持）
///
/// ## 基本用法
///
/// ```
/// let client = LlmClient::from_preset("antigravity")
///     .expect("Preset should exist")
///     .with_antigravity_oauth("~/.config/antigravity/token.json")
///     .build();
/// ```
///
/// ## 支持的模型
///
/// | 模型 | 说明 |
/// |------|------|
/// | gemini-2.5-flash | Gemini 快速模型 |
/// | gemini-2.5-pro | Gemini 专业模型 |
/// | claude-sonnet-4-6 | Claude Sonnet（通过翻译层）|
/// | claude-opus-4-6-thinking | Claude Opus Thinking（通过翻译层）|
///
/// ## 特殊说明
///
/// - Claude 模型通过翻译层转换为 Gemini 格式
/// - 使用 `fetchAvailableModels` 端点获取完整模型列表
/// - 支持 Gemini 3.x 预览版模型
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CloudCodeSite::new())
        .protocol(GeminiProtocol {})
        .with_protocol_hook(Arc::new(CloudCodeHook {}))
        .with_extension(Arc::new(AntigravityExtension {}))
        .model_resolver(AntigravityModelResolver::new())
        .default_model("gemini-2.5-flash")
}
