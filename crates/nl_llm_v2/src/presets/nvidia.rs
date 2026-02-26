use crate::client::ClientBuilder;
use crate::model::nvidia::NvidiaModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::nvidia::NvidiaExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// NVIDIA NIM (NVIDIA Inference Microservices) 预设
///
/// # 平台特性
///
/// - **网关**: `https://integrate.api.nvidia.com/v1`
/// - **认证**: Bearer 格式，API Key 以 `nvapi-` 为前缀
/// - **协议**: OpenAI 兼容
/// - **模型数量**: 186+ 个开源和 NVIDIA 优化模型
///
/// # 支持的模型（部分）
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `meta/llama-3.3-70b-instruct` | 128K | Llama 3.3 70B |
/// | `meta/llama-3.1-405b-instruct` | 128K | Llama 3.1 405B |
/// | `nvidia/llama-3.1-nemotron-70b-instruct` | 128K | NVIDIA Nemotron |
/// | `deepseek-ai/deepseek-r1` | 64K | DeepSeek R1，支持思考 |
/// | `qwen/qwen2.5-72b-instruct` | 128K | Qwen 2.5 72B |
/// | `google/gemma-2-27b-it` | 8K | Gemma 2 27B |
///
/// # 模型别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `nvidia` / `llama` | `meta/llama-3.3-70b-instruct` |
/// | `llama-405b` | `meta/llama-3.1-405b-instruct` |
/// | `nemotron` | `nvidia/llama-3.1-nemotron-70b-instruct` |
/// | `deepseek` / `r1` | `deepseek-ai/deepseek-r1` |
/// | `qwen` | `qwen/qwen2.5-72b-instruct` |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("nvidia")
///     .expect("Preset should exist")
///     .with_api_key("nvapi-xxx")
///     .build();
/// ```
///
/// # 获取 API Key
///
/// 访问 https://build.nvidia.com/ 获取 API Key
const NVIDIA_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(NVIDIA_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(NvidiaModelResolver::new())
        .with_extension(Arc::new(
            NvidiaExtension::new().with_base_url(NVIDIA_BASE_URL),
        ))
        .default_model("meta/llama-3.3-70b-instruct")
}
