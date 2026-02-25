use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::QwenModelResolver;
use crate::provider::qwen::QwenExtension;

/// 阿里云通义千问 (Qwen) API 预设
///
/// 通义千问是阿里云推出的大语言模型系列，支持多种规模和能力。
/// 本预设使用阿里云百炼平台的 DashScope 兼容 API。
///
/// # 平台特性
///
/// - **端点**: `https://dashscope.aliyuncs.com/compatible-mode/v1`
/// - **认证**: `Authorization: Bearer <QWEN_API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 模型说明
///
/// - **`qwen-max`**: 旗舰通用大模型
/// - **`qwen-plus`**: 均衡通用大模型（默认）
/// - **`qwen-turbo`**: 极速通用大模型
/// - **`qwen2.5-coder-32b-instruct`**: 顶尖开源代码生成模型
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `qwen` / `plus` | `qwen-plus` | 默认模型 |
/// | `max` | `qwen-max` | 旗舰模型 |
/// | `coder` / `code` | `qwen2.5-coder-32b-instruct` | 代码生成 |
/// | `vl` / `vision` | `qwen-vl-max` | 视觉多模态 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("qwen")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .with_concurrency()  // 可选：启用并发控制
///     .build();
///
/// // 使用模型别名
/// let req = PrimitiveRequest::single_user_message("Write a hello world in Rust")
///     .with_model("coder");  // 自动解析为 qwen2.5-coder-32b-instruct
/// ```
const QWEN_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(QWEN_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(QwenModelResolver::new())
        .with_extension(Arc::new(QwenExtension::new().with_base_url(QWEN_BASE_URL)))
        .default_model("qwen-plus")  // 默认使用性价比最高且能力均衡的 qwen-plus
}
