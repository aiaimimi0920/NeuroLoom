/// 讯飞星火 (iFlytek Spark) 通用预设
///
/// # 平台特性
///
/// - **网关**: `https://spark-api-open.xf-yun.com/v1`
/// - **认证**: `Authorization: Bearer <APIPassword>`
/// - **协议**: OpenAI 兼容
/// - **适用模型**: Lite, Pro, Pro-128K, Max, Max-32K, 4.0Ultra
///
/// # ⚠️ 认证方式
///
/// 需要从控制台获取 `APIPassword`（HTTP 认证专用），不是 WebSocket 的 APIKey/APISecret。
///
/// ```rust,no_run
/// let client = nl_llm_v2::LlmClient::from_preset("spark")
///     .expect("Preset should exist")
///     .with_spark_auth("your_api_password")
///     .build();
/// ```
use crate::client::ClientBuilder;
use crate::model::spark::SparkModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::spark::SparkExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

const SPARK_BASE_URL: &str = "https://spark-api-open.xf-yun.com/v1";

/// 通用预设 (Lite / Pro / Max / Ultra)
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(SPARK_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(SparkModelResolver::new())
        .with_extension(Arc::new(SparkExtension::new()))
        .default_model("lite")
}

/// Spark X 专用预设
///
/// - **网关**: `https://spark-api-open.xf-yun.com/x2`
/// - **默认模型**: `x2` (Spark X2)
///
/// 注意: Spark X1.5 使用 `/v2` 端点，X2 使用 `/x2` 端点
const SPARK_X_BASE_URL: &str = "https://spark-api-open.xf-yun.com/x2";

pub fn builder_x() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(SPARK_X_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(SparkModelResolver::new())
        .with_extension(Arc::new(SparkExtension::new()))
        .default_model("spark-x")
}
