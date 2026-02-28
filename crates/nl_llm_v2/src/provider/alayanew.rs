use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::model::default::DefaultModelResolver;
use crate::model::resolver::{Capability, Modality, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

const DEEPSEEK_CHAT: &str = "deepseek-chat";
const DEEPSEEK_REASONER: &str = "deepseek-reasoner";

pub struct AlayanewModelResolver {
    inner: DefaultModelResolver,
}

impl AlayanewModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("alayanew", DEEPSEEK_CHAT),
            ("deepseek", DEEPSEEK_CHAT),
            ("ds", DEEPSEEK_CHAT),
            ("v3", DEEPSEEK_CHAT),
            ("r1", DEEPSEEK_REASONER),
            ("reasoner", DEEPSEEK_REASONER),
            ("think", DEEPSEEK_REASONER),
        ]);

        inner.extend_capabilities(vec![
            (
                DEEPSEEK_CHAT,
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                DEEPSEEK_REASONER,
                Capability::CHAT | Capability::THINKING | Capability::STREAMING,
            ),
        ]);

        inner.extend_context_lengths(vec![(DEEPSEEK_CHAT, 65_536), (DEEPSEEK_REASONER, 65_536)]);

        inner.extend_intelligence_profiles(vec![
            (DEEPSEEK_CHAT, 4.0, Modality::Text),
            (DEEPSEEK_REASONER, 4.2, Modality::Text),
        ]);

        Self { inner }
    }
}

impl Default for AlayanewModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AlayanewModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            DEEPSEEK_CHAT.to_string()
        } else {
            self.inner.resolve(model)
        }
    }

    fn has_capability(&self, model: &str, capability: Capability) -> bool {
        self.inner.has_capability(model, capability)
    }

    fn max_context(&self, model: &str) -> usize {
        self.inner.max_context(model)
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        match self.resolve(model).as_str() {
            DEEPSEEK_CHAT | DEEPSEEK_REASONER => (49_152, 16_384),
            _ => (8192, 4096),
        }
    }

    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, Modality)> {
        self.inner
            .intelligence_and_modality(model)
            .or(Some((3.5, Modality::Text)))
    }
}

pub struct AlayanewExtension {
    base_url: String,
}

impl AlayanewExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: DEEPSEEK_CHAT.to_string(),
                description: "DeepSeek Chat V3 (AlayaNew)".to_string(),
            },
            ModelInfo {
                id: DEEPSEEK_REASONER.to_string(),
                description: "DeepSeek Reasoner R1 (AlayaNew)".to_string(),
            },
        ]
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelItem>,
}

#[derive(Deserialize)]
struct OpenAiModelItem {
    id: String,
}

#[async_trait]
impl ProviderExtension for AlayanewExtension {
    fn id(&self) -> &str {
        "alayanew"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get(self.models_url());
        let req = auth.inject(req)?;

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(payload) = resp.json::<OpenAiModelsResponse>().await {
                    let models: Vec<ModelInfo> = payload
                        .data
                        .into_iter()
                        .filter(|m| !m.id.trim().is_empty())
                        .map(|m| ModelInfo {
                            description: format!("{} (AlayaNew)", m.id),
                            id: m.id,
                        })
                        .collect();
                    if !models.is_empty() {
                        return Ok(models);
                    }
                }
            }
        }

        Ok(Self::fallback_models())
    }

    fn concurrency_config(&self) -> crate::concurrency::ConcurrencyConfig {
        crate::concurrency::ConcurrencyConfig::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alayanew_resolver_supports_aliases() {
        let resolver = AlayanewModelResolver::new();
        assert_eq!(resolver.resolve(""), DEEPSEEK_CHAT);
        assert_eq!(resolver.resolve("deepseek"), DEEPSEEK_CHAT);
        assert_eq!(resolver.resolve("r1"), DEEPSEEK_REASONER);
    }

    #[test]
    fn alayanew_resolver_capability_matches_model_type() {
        let resolver = AlayanewModelResolver::new();
        assert!(resolver.has_capability("deepseek-chat", Capability::TOOLS));
        assert!(resolver.has_capability("r1", Capability::THINKING));
        assert!(!resolver.has_capability("deepseek-chat", Capability::THINKING));
    }
}
