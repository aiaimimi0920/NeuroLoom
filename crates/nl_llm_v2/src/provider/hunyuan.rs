use async_trait::async_trait;
use serde_json::{json, Value};

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, DefaultModelResolver, ModelResolver};
use crate::pipeline::traits::{PipelineContext, PipelineInput};
use crate::protocol::traits::ProtocolHook;
use crate::provider::extension::{ModelInfo, ProviderExtension};

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

        // 模型能力与上下文窗口集中定义，避免 resolver / extension 漂移
        inner.extend_capabilities(
            hunyuan_model_metadata()
                .iter()
                .map(|(id, _, _, cap)| (*id, *cap))
                .collect::<Vec<_>>(),
        );

        inner.extend_context_lengths(
            hunyuan_model_metadata()
                .iter()
                .map(|(id, _, context, _)| (*id, *context))
                .collect::<Vec<_>>(),
        );

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

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        self.inner.intelligence_and_modality(model)
    }
}

/// 腾讯混元协议拦截器
/// 负责在发送 OpenAI 标准协议 JSON 前，注入混元特有的参数（如 enable_enhancement: true）
pub struct HunyuanHook;

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
        Ok(hunyuan_model_metadata()
            .iter()
            .map(|(id, description, _, _)| ModelInfo {
                id: (*id).to_string(),
                description: (*description).to_string(),
            })
            .collect())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 官方未公开明确并发上限；这里提供保守的默认值，配合 AIMD 自适应探测。
        ConcurrencyConfig {
            official_max: 20,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 30,
            ..Default::default()
        }
    }
}

impl ProtocolHook for HunyuanHook {
    fn after_pack(&self, ctx: &mut PipelineContext, packed: &mut Value) {
        if let PipelineInput::Primitive(_) = &ctx.input {
            // Hunyuan 支持增强开关：默认开启，但尊重调用方显式传入值。
            if packed.get("enable_enhancement").is_none() {
                packed["enable_enhancement"] = json!(true);
            }
        }
    }
}

fn hunyuan_model_metadata() -> Vec<(&'static str, &'static str, usize, Capability)> {
    vec![
        (
            "hunyuan-turbos-latest",
            "Tencent Hunyuan Turbo Stream Latest",
            128_000,
            Capability::CHAT | Capability::STREAMING | Capability::TOOLS,
        ),
        (
            "hunyuan-pro",
            "Tencent Hunyuan Pro",
            32_000,
            Capability::CHAT | Capability::STREAMING | Capability::TOOLS,
        ),
        (
            "hunyuan-standard",
            "Tencent Hunyuan Standard",
            128_000,
            Capability::CHAT | Capability::STREAMING | Capability::TOOLS,
        ),
        (
            "hunyuan-vision",
            "Tencent Hunyuan Vision",
            8_000,
            Capability::CHAT | Capability::STREAMING | Capability::VISION,
        ),
        (
            "hunyuan-code",
            "Tencent Hunyuan Code",
            32_000,
            Capability::CHAT | Capability::STREAMING,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::PrimitiveRequest;
    use crate::site::context::{Action, AuthType, UrlContext};

    fn test_ctx(req: PrimitiveRequest) -> PipelineContext<'static> {
        PipelineContext::from_primitive(
            req,
            UrlContext {
                model: "hunyuan",
                auth_type: AuthType::ApiKey,
                action: Action::Generate,
                tenant: None,
            },
        )
    }

    #[test]
    fn hook_sets_default_enhancement() {
        let hook = HunyuanHook;
        let req = PrimitiveRequest::single_user_message("hello").with_model("hunyuan");
        let mut ctx = test_ctx(req);
        let mut packed = json!({"model": "hunyuan"});

        hook.after_pack(&mut ctx, &mut packed);

        assert_eq!(packed["enable_enhancement"], json!(true));
    }

    #[test]
    fn hook_keeps_user_defined_enhancement() {
        let hook = HunyuanHook;
        let req = PrimitiveRequest::single_user_message("hello").with_model("hunyuan");
        let mut ctx = test_ctx(req);
        let mut packed = json!({
            "model": "hunyuan",
            "enable_enhancement": false
        });

        hook.after_pack(&mut ctx, &mut packed);

        assert_eq!(packed["enable_enhancement"], json!(false));
    }

    #[test]
    fn resolver_supports_known_alias_and_capability() {
        let resolver = HunyuanModelResolver::new();
        assert_eq!(resolver.resolve("hunyuan-turbo"), "hunyuan-turbos-latest");
        assert!(resolver.has_capability("hunyuan-vision", Capability::VISION));
    }
}
