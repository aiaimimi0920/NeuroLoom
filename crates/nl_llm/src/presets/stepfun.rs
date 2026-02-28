use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::custom::{CustomExtension, CustomModelResolver};
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// 阶跃星辰 StepFun API 预设
///
/// 阶跃星辰是国内领先的大模型创业公司，提供 Step 系列大语言模型。
/// 使用标准 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.stepfun.com/v1`（国内）
/// - **认证**: `Authorization: Bearer <STEPFUN_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **特色**: Step 系列模型，支持超长上下文和多模态
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("stepfun")
///     .expect("Preset should exist")
///     .with_api_key("your_key")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("step-2-16k");
/// ```
const STEPFUN_BASE_URL: &str = "https://api.stepfun.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(STEPFUN_BASE_URL))
        .protocol(OpenAiProtocol)
        // StepFun 模型名更新较快，使用动态模型解析与 /models 拉取，避免预置列表过时。
        .model_resolver(CustomModelResolver::new())
        .with_extension(Arc::new(CustomExtension::new(STEPFUN_BASE_URL)))
        .default_model("step-2-16k")
}
