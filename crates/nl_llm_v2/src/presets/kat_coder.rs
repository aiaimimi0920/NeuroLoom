use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::KatCoderModelResolver;
use crate::provider::kat_coder::KatCoderExtension;

/// KAT-Coder (StreamLake / Vanchin) API 预设
///
/// 快手旗下 StreamLake 平台推出的代码大模型服务，
/// 通过 Vanchin 网关提供 OpenAI 兼容 API。
///
/// # 平台特性
///
/// - **网关**: `https://vanchin.streamlake.ai/api/gateway/v1/endpoints`
/// - **认证**: `Authorization: Bearer <API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **定位**: 代码生成专精，支持 128K 上下文
///
/// # 模型说明
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `KAT-Coder-Pro` | Chat, Tools, Streaming | 128K | 旗舰代码模型 |
/// | `KAT-Coder-Pro-V1` | Chat, Tools, Streaming | 128K | 旗舰版本 V1（Claude 代理模式）|
/// | `KAT-Coder-Air-V1` | Chat, Tools, Streaming | 128K | 轻量级代码模型 |
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `kat_coder` / `pro` | `KAT-Coder-Pro` | 默认旗舰模型 |
/// | `pro-v1` | `KAT-Coder-Pro-V1` | 旗舰版本 V1 |
/// | `air` / `air-v1` | `KAT-Coder-Air-V1` | 轻量快速模型 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("kat_coder")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .with_concurrency()  // 可选：启用并发控制
///     .build();
///
/// // 使用别名
/// let req = PrimitiveRequest::single_user_message("写一个快速排序")
///     .with_model("pro");  // 自动解析为 KAT-Coder-Pro
///
/// // 使用轻量模型
/// let req = PrimitiveRequest::single_user_message("写一个快速排序")
///     .with_model("air");  // 自动解析为 KAT-Coder-Air-V1
/// ```
const KAT_CODER_BASE_URL: &str = "https://vanchin.streamlake.ai/api/gateway/v1/endpoints";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(KAT_CODER_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(KatCoderModelResolver::new())
        .with_extension(Arc::new(KatCoderExtension::new().with_base_url(KAT_CODER_BASE_URL)))
        .default_model("kat-coder-pro")
}
