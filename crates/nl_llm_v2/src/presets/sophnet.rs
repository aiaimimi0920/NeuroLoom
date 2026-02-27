use std::sync::Arc;

use crate::auth::providers::{ApiKeyAuth, MultiKeyAuth, MultiKeyMode};
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::sophnet::{SophnetExtension, SophnetModelResolver};
use crate::site::base::openai::OpenAiSite;

pub fn builder() -> ClientBuilder {
    let base_url =
        std::env::var("SOPHNET_BASE_URL").unwrap_or_else(|_| "https://www.sophnet.com/api/open-apis/v1".to_string());

    let mut builder = ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(SophnetModelResolver::new())
        .with_extension(Arc::new(SophnetExtension::new(base_url)))
        .default_model("DeepSeek-v3");

    if let Some(multi_key_auth) = read_multi_key_auth_from_env() {
        builder = builder.auth(multi_key_auth);
    } else if let Ok(api_key) = std::env::var("SOPHNET_API_KEY") {
        builder = builder.auth(ApiKeyAuth::new(api_key));
    }

    builder
}

fn read_multi_key_auth_from_env() -> Option<MultiKeyAuth> {
    let raw = std::env::var("SOPHNET_API_KEYS").ok().or_else(|| std::env::var("SOPHNET_API_KEY").ok())?;
    let keys: Vec<String> = raw
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();

    if keys.is_empty() {
        return None;
    }

    let mode = match std::env::var("SOPHNET_MULTI_KEY_MODE")
        .unwrap_or_else(|_| "round_robin".to_string())
        .to_lowercase()
        .as_str()
    {
        "random" => MultiKeyMode::Random,
        _ => MultiKeyMode::RoundRobin,
    };

    Some(MultiKeyAuth::new(keys).with_mode(mode))
}

impl LlmClient {
    pub fn build_sophnet(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(SophnetExtension::new(&base_url)))
            .build()
    }

    pub fn build_sophnet_multi(
        base_url: impl Into<String>,
        api_keys: Vec<String>,
        mode: MultiKeyMode,
    ) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(MultiKeyAuth::new(api_keys).with_mode(mode))
            .with_extension(Arc::new(SophnetExtension::new(&base_url)))
            .build()
    }
}
