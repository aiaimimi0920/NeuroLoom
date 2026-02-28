use crate::client::ClientBuilder;
use crate::model::modelscope::ModelScopeModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::modelscope::ModelScopeExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// ModelScope (魔搭) 预设
///
/// # 平台特性
///
/// - **网关**: `https://api-inference.modelscope.cn/v1`
/// - **认证**: ModelScope Access Token，标准 Bearer 格式
/// - **协议**: OpenAI 兼容
/// - **获取 Token**: <https://modelscope.cn/my/myaccesstoken>
///
/// # 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `Qwen/Qwen3-235B-A22B` | 128K | Qwen3 旗舰 MoE，支持思考 |
/// | `Qwen/Qwen3-32B` | 128K | Qwen3 32B Dense |
/// | `Qwen/Qwen3-8B` | 128K | Qwen3 8B 轻量 |
/// | `Qwen/Qwen2.5-Coder-32B-Instruct` | 128K | 编程专用 |
/// | `Qwen/QVQ-72B-Preview` | 128K | 视觉推理 |
///
/// # 模型别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `modelscope` / `qwen3` | `Qwen/Qwen3-235B-A22B` |
/// | `qwen3-32b` | `Qwen/Qwen3-32B` |
/// | `qwen3-8b` | `Qwen/Qwen3-8B` |
/// | `coder` | `Qwen/Qwen2.5-Coder-32B-Instruct` |
/// | `qvq` | `Qwen/QVQ-72B-Preview` |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("modelscope")
///     .expect("Preset should exist")
///     .with_api_key("your-modelscope-access-token")
///     .build();
/// ```
const MODELSCOPE_BASE_URL: &str = "https://api-inference.modelscope.cn/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(MODELSCOPE_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(ModelScopeModelResolver::new())
        .with_extension(Arc::new(
            ModelScopeExtension::new().with_base_url(MODELSCOPE_BASE_URL),
        ))
        .default_model("Qwen/Qwen3-235B-A22B")
}
