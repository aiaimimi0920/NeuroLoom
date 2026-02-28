use crate::client::ClientBuilder;
use crate::model::default::DefaultModelResolver;
use crate::model::resolver::{Capability, ModelResolver};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// Vercel AI Gateway API 预设
///
/// Vercel AI Gateway 是 Vercel 提供的统一 AI 接口网关，
/// 支持路由请求到多个模型提供商，并自带速率限制、缓存等功能。
///
/// # 平台特性
///
/// - **端点**: `https://ai-gateway.vercel.sh/v1`
/// - **认证**: `Authorization: Bearer <VERCEL_AI_GATEWAY_API_KEY>`
/// - **协议**: 官方提供 OpenAI 兼容的统一网关协议端点
/// - **模型格式**: 推荐使用 `provider/model`（例如 `openai/gpt-4o-mini`）
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("vercel_ai_gateway")
///     .expect("Preset should exist")
///     .with_api_key("vck_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("openai/gpt-4o-mini"); // provider/model 形式更符合网关路由
/// ```
const VERCEL_AI_GATEWAY_BASE_URL: &str = "https://ai-gateway.vercel.sh/v1";

/// Vercel AI Gateway 模型解析器
///
/// Vercel 网关建议使用 provider/model 命名。这里为常见 OpenAI 简写提供别名，
/// 既兼容历史写法，也能对齐最新文档推荐的命名规范。
pub struct VercelAiGatewayModelResolver {
    inner: DefaultModelResolver,
}

impl VercelAiGatewayModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("gpt-4o", "openai/gpt-4o"),
            ("gpt-4o-mini", "openai/gpt-4o-mini"),
            ("gpt-4.1", "openai/gpt-4.1"),
            ("gpt-4.1-mini", "openai/gpt-4.1-mini"),
        ]);

        inner.extend_capabilities(vec![
            (
                "openai/gpt-4o",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "openai/gpt-4o-mini",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "openai/gpt-4.1",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "openai/gpt-4.1-mini",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
        ]);

        inner.extend_context_lengths(vec![
            ("openai/gpt-4o", 128_000),
            ("openai/gpt-4o-mini", 128_000),
            ("openai/gpt-4.1", 1_000_000),
            ("openai/gpt-4.1-mini", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for VercelAiGatewayModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for VercelAiGatewayModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.inner.resolve(model)
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        self.inner.has_capability(model, cap)
    }

    fn max_context(&self, model: &str) -> usize {
        self.inner.max_context(model)
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        self.inner.context_window_hint(model)
    }

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        self.inner.intelligence_and_modality(model)
    }
}

pub fn builder() -> ClientBuilder {
    let base_url = std::env::var("VERCEL_AI_GATEWAY_BASE_URL")
        .unwrap_or_else(|_| VERCEL_AI_GATEWAY_BASE_URL.to_string());

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol)
        .model_resolver(VercelAiGatewayModelResolver::new())
        .default_model("openai/gpt-4o")
}
