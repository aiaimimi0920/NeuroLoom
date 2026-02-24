use crate::client::ClientBuilder;
use crate::site::base::iflow::IFlowSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::protocol::hooks::iflow::IflowThinkingHook;
use crate::model::iflow::IFlowModelResolver;
use crate::provider::iflow::IFlowExtension;
use std::sync::Arc;

/// iFlow (心流) API 预设
///
/// iFlow 是一个 AI 聚合平台，提供多种开源和闭源模型的统一接入。
///
/// ## 基本信息
///
/// - 官网：https://iflow.cn
/// - API 端点：`https://apis.iflow.cn/v1`
/// - 认证方式：Cookie (BXAuth)
///
/// ## 基本用法
///
/// ```
/// let client = LlmClient::from_preset("iflow")
///     .expect("Preset should exist")
///     .with_cookie("BXAuth=xxx")
///     .build();
/// ```
///
/// ## 支持的模型
///
/// | 模型 | 说明 |
/// |------|------|
/// | qwen3-max | 通义千问最强版 |
/// | deepseek-v3.2 | DeepSeek V3 |
/// | glm-4-plus | 智谱 GLM-4 |
///
/// ## Thinking 模式
///
/// iFlow 支持 Thinking 模式，会自动为支持的模型启用深度思考：
/// - 自动检测模型名称中是否包含 "thinking" 或 "r1"
/// - 自动添加 `chat_template_kwargs.enable_thinking` 参数
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(IFlowSite::new())
        .protocol(OpenAiProtocol {})
        .with_protocol_hook(Arc::new(IflowThinkingHook {}))
        .with_extension(Arc::new(IFlowExtension {}))
        .model_resolver(IFlowModelResolver::new())
        .default_model("qwen3-max")
}
