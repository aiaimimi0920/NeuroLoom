use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::QwenModelResolver;
use crate::provider::qwen::QwenExtension;

/// 通义千问 Qwen2.5-Coder (编程专家级) 预设
///
/// 从大而全的通义千问 `qwen` 平台分离出来的纯代码预置方案。
///
/// 采用阿里专为编程任务极致优化发布的 `qwen2.5-coder-32b-instruct` 模型。由于直接提供专项服务，
/// 它是你搭建例如游戏编辑器、VSCode 插件 AI 时首选的预设通道。
const QWEN_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(QWEN_BASE_URL))
        .protocol(OpenAiProtocol {})
        // 继续沿用千问体系的统一标识符转换器
        .model_resolver(QwenModelResolver::new())
        .with_extension(Arc::new(QwenExtension::new()))
        // 明确将默认模型重定向为最强大的代码模型
        .default_model("qwen2.5-coder-32b-instruct")
}
