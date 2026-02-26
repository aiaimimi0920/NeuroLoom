use crate::client::ClientBuilder;
use crate::model::KimiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::kimi::KimiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// 月之暗面 Kimi (Moonshot) API 预设
///
/// Kimi 是月之暗面科技推出的大模型，擅长长文本处理。
/// 本预设默认支持其公网标准接入点，并通过 OpenAI 兼容层使用该服务。
///
/// # 平台特性
///
/// - **端点1 (常规)**: `https://api.moonshot.cn/v1` （默认）
/// - **端点2 (为代码)**: `https://api.kimi.com/v1` （可通过 `.with_base_url` 切换）
/// - **认证**: `Authorization: Bearer <KIMI_API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 模型说明
///
/// - **`kimi-k2.5`**: 最新最强版本
/// - **`moonshot-v1-32k`**: 前代稳定版本
/// - **`kimi-for-coding`**: 专门用于代码生成的模型
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `kimi` / `k2.5` | `kimi-k2.5` | 默认最新主力 |
/// | `coding` / `code` | `kimi-for-coding` | 建议配置至 `api.kimi.com` 调用 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("kimi")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
///
/// // 普通对话
/// let req = PrimitiveRequest::single_user_message("你好!");
///
/// // 专攻代码对话 (可能需要切换至 api.kimi.com 或者直接可用由官方路由决定)
/// let req_code = PrimitiveRequest::single_user_message("写个排序")
///     .with_model("coding");
/// ```
const KIMI_BASE_URL: &str = "https://api.moonshot.cn/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(KIMI_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(KimiModelResolver::new())
        .with_extension(Arc::new(KimiExtension::new().with_base_url(KIMI_BASE_URL)))
        .default_model("kimi-k2.5") // K2.5 作为默认的首选最强入口
}
