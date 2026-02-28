use crate::client::ClientBuilder;
use crate::model::siliconflow::SiliconFlowModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::siliconflow::SiliconFlowExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// SiliconFlow (硅基流动) 预设
///
/// # 平台特性
///
/// - **网关**: `https://api.siliconflow.cn/v1`
/// - **认证**: 标准 Bearer 格式
/// - **协议**: OpenAI 兼容
///
/// # 模型命名规则
///
/// - `Pro/组织/模型` — 高性能推理层
/// - `组织/模型` — 标准推理层
/// - `Free/组织/模型` — 免费推理层
///
/// # 使用示例
///
/// ```rust
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("siliconflow")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
const SILICONFLOW_BASE_URL: &str = "https://api.siliconflow.cn/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(SILICONFLOW_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(SiliconFlowModelResolver::new())
        .with_extension(Arc::new(
            SiliconFlowExtension::new().with_base_url(SILICONFLOW_BASE_URL),
        ))
        .default_model("Pro/moonshotai/Kimi-K2.5")
}
