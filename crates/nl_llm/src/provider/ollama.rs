use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};

const OLLAMA_DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434/v1";
const OLLAMA_DEFAULT_MODEL: &str = "llama3";

pub struct OllamaExtension {
    base_url: String,
}

impl OllamaExtension {
    pub fn new() -> Self {
        Self {
            base_url: OLLAMA_DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    fn openai_models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }

    fn native_tags_url(&self) -> String {
        let root = self.base_url.trim_end_matches("/v1");
        format!("{}/api/tags", root)
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![ModelInfo {
            id: OLLAMA_DEFAULT_MODEL.to_string(),
            description: "Local Default".to_string(),
        }]
    }

    async fn list_openai_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<Vec<ModelInfo>>> {
        let mut req = http.get(self.openai_models_url());
        req = auth.inject(req)?;
        let res = req.send().await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let list: OpenAiModelList = match res.json().await {
            Ok(data) => data,
            Err(_) => return Ok(None),
        };

        let models = list
            .data
            .into_iter()
            .map(|model| ModelInfo {
                description: model.id.clone(),
                id: model.id,
            })
            .collect::<Vec<_>>();

        if models.is_empty() {
            Ok(None)
        } else {
            Ok(Some(models))
        }
    }

    async fn list_native_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<Vec<ModelInfo>>> {
        let mut req = http.get(self.native_tags_url());
        req = auth.inject(req)?;
        let res = req.send().await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let list: NativeModelList = match res.json().await {
            Ok(data) => data,
            Err(_) => return Ok(None),
        };

        let models = list
            .models
            .into_iter()
            .map(|model| ModelInfo {
                description: model.name.clone(),
                id: model.name,
            })
            .collect::<Vec<_>>();

        if models.is_empty() {
            Ok(None)
        } else {
            Ok(Some(models))
        }
    }
}

impl Default for OllamaExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiModelList {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

#[derive(Debug, Deserialize)]
struct NativeModelList {
    models: Vec<NativeModel>,
}

#[derive(Debug, Deserialize)]
struct NativeModel {
    name: String,
}

#[async_trait::async_trait]
impl ProviderExtension for OllamaExtension {
    fn id(&self) -> &str {
        "ollama"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        if let Some(models) = self.list_openai_models(http, auth).await? {
            return Ok(models);
        }

        if let Some(models) = self.list_native_models(http, auth).await? {
            return Ok(models);
        }

        Ok(Self::fallback_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<crate::provider::balance::BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 2,
            initial_limit: 1,
            min_limit: 1,
            max_limit: 4,
            ..Default::default()
        }
    }
}
