use crate::pipeline::traits::{PipelineContext, PipelineInput};
use crate::model::{Capability, DefaultModelResolver, ModelResolver};
use crate::protocol::traits::ProtocolHook;
use serde_json::{json, Value};

/// 腾讯混元模型解析器
pub struct HunyuanModelResolver {
    inner: DefaultModelResolver,
}

impl Default for HunyuanModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl HunyuanModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // 混元特有别名映射
        inner.extend_aliases(vec![
            ("hunyuan", "hunyuan-turbos-latest"),
            ("hunyuan-turbo", "hunyuan-turbos-latest"),
            ("hunyuan-pro", "hunyuan-pro"),
            ("hunyuan-standard", "hunyuan-standard"),
            ("hunyuan-vision", "hunyuan-vision"),
            ("hunyuan-code", "hunyuan-code"),
        ]);

        // 混元模型能力
        inner.extend_capabilities(vec![
            (
                "hunyuan-turbos-latest",
                Capability::CHAT | Capability::STREAMING | Capability::TOOLS,
            ),
            (
                "hunyuan-pro",
                Capability::CHAT | Capability::STREAMING | Capability::TOOLS,
            ),
            (
                "hunyuan-standard",
                Capability::CHAT | Capability::STREAMING | Capability::TOOLS,
            ),
            (
                "hunyuan-vision",
                Capability::CHAT | Capability::STREAMING | Capability::VISION,
            ),
            ("hunyuan-code", Capability::CHAT | Capability::STREAMING),
        ]);

        // 上下文长度 (基于一般大模型标准，具体可查阅官方文档)
        inner.extend_context_lengths(vec![
            ("hunyuan-turbos-latest", 128_000),
            ("hunyuan-pro", 32_000),
            ("hunyuan-standard", 128_000),
            ("hunyuan-vision", 8_000),
            ("hunyuan-code", 32_000),
        ]);

        Self { inner }
    }
}

impl ModelResolver for HunyuanModelResolver {
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
}

/// 腾讯混元协议拦截器
/// 负责在发送 OpenAI 标准协议 JSON 前，注入混元特有的参数（如 enable_enhancement: true）
pub struct HunyuanHook;

use async_trait::async_trait;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::auth::traits::Authenticator;

#[async_trait]
impl ProviderExtension for HunyuanHook {
    fn id(&self) -> &str {
        "hunyuan"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "hunyuan-turbos-latest".to_string(),
                description: "Tencent Hunyuan Turbo Stream Latest".to_string(),
            },
            ModelInfo {
                id: "hunyuan-pro".to_string(),
                description: "Tencent Hunyuan Pro".to_string(),
            },
            ModelInfo {
                id: "hunyuan-standard".to_string(),
                description: "Tencent Hunyuan Standard".to_string(),
            },
            ModelInfo {
                id: "hunyuan-vision".to_string(),
                description: "Tencent Hunyuan Vision".to_string(),
            },
            ModelInfo {
                id: "hunyuan-code".to_string(),
                description: "Tencent Hunyuan Code".to_string(),
            },
        ])
    }
}

impl ProtocolHook for HunyuanHook {
    fn after_pack(&self, ctx: &mut PipelineContext, packed: &mut Value) {
        if let PipelineInput::Primitive(_) = &ctx.input {
            // Hunyuan 需要在 body 中添加 enable_enhancement: true
            packed["enable_enhancement"] = json!(true);
        }
    }
}
