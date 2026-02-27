use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// Hugging Face API 预设
///
/// 使用 Hugging Face Inference API 的 OpenAI 兼容路由端点。
/// 支持无缝调用部署在 Hugging Face 上的万千开源模型。
///
/// # 平台特性
///
/// - **端点**: `https://router.huggingface.co/v1`
/// - **认证**: `Authorization: Bearer <HF_TOKEN>`
/// - **协议**: OpenAI 兼容
/// - **特色**: 生态庞大，允许调用诸多社区微调模型
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("huggingface")
///     .expect("Preset should exist")
///     .with_api_key("hf_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("meta-llama/Llama-2-7b-chat-hf");
/// ```
const HUGGINGFACE_BASE_URL: &str = "https://router.huggingface.co/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(HUGGINGFACE_BASE_URL))
        .protocol(OpenAiProtocol)
        .default_model("meta-llama/Llama-3.3-70B-Instruct")
}
