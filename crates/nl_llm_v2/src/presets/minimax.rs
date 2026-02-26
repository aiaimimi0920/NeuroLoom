use crate::client::ClientBuilder;
use crate::model::minimax::MiniMaxModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::minimax::MiniMaxExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// MiniMax 英文站预设
///
/// # 平台特性
///
/// - **网关**: `https://api.minimax.io/v1`
/// - **认证**: `Authorization: Bearer <API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `MiniMax-M2.5` | 200K | 旗舰模型，支持 CoT 思考 |
/// | `MiniMax-M2.5-highspeed` | 200K | 旗舰高速版 |
/// | `MiniMax-M2.1` | 200K | 编程增强版，支持 CoT 思考 |
/// | `MiniMax-M2.1-highspeed` | 200K | 编程增强高速版 |
/// | `MiniMax-M2` | 128K | 标准模型 |
/// | `M2-her` | 128K | 多角色扮演模型 |
///
/// # 模型别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `minimax` / `m2.5` | `MiniMax-M2.5` |
/// | `m2.5-fast` | `MiniMax-M2.5-highspeed` |
/// | `m2.1` | `MiniMax-M2.1` |
/// | `m2.1-fast` | `MiniMax-M2.1-highspeed` |
/// | `m2` | `MiniMax-M2` |
/// | `her` | `M2-her` |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("minimax")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
///
/// # 相关预设
///
/// - `minimax_cn`: MiniMax 中国站 (`api.minimaxi.com`)
const MINIMAX_BASE_URL: &str = "https://api.minimax.io/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(MINIMAX_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(MiniMaxModelResolver::new())
        .with_extension(Arc::new(
            MiniMaxExtension::new().with_base_url(MINIMAX_BASE_URL),
        ))
        .default_model("MiniMax-M2.5")
}
