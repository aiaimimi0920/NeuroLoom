use std::sync::Arc;

use crate::auth::providers::{ApiKeyAuth, MultiKeyAuth, MultiKeyMode};
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::ph8::{Ph8Extension, Ph8ModelResolver};
use crate::site::base::openai::OpenAiSite;

pub fn builder() -> ClientBuilder {
    let base_url = std::env::var("PH8_BASE_URL")
        .unwrap_or_else(|_| "https://ph8.co/v1".to_string());

    let mut builder = ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(Ph8ModelResolver::new())
        .with_extension(Arc::new(Ph8Extension::new(base_url)))
        .default_model("qwen-max");

    if let Some(multi_key_auth) = read_multi_key_auth_from_env() {
        builder = builder.auth(multi_key_auth);
    }

    builder
}

fn read_multi_key_auth_from_env() -> Option<MultiKeyAuth> {
    let raw = std::env::var("PH8_API_KEYS").ok().or_else(|| std::env::var("PH8_API_KEY").ok())?;
    let keys: Vec<String> = raw
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();

    if keys.is_empty() {
        return None;
    }

    let mode = match std::env::var("PH8_MULTI_KEY_MODE")
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
    pub fn build_ph8(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(Ph8Extension::new(&base_url)))
            .build()
    }

    pub fn build_ph8_multi(
        base_url: impl Into<String>,
        api_keys: Vec<String>,
        mode: MultiKeyMode,
    ) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(MultiKeyAuth::new(api_keys).with_mode(mode))
            .with_extension(Arc::new(Ph8Extension::new(&base_url)))
            .build()
    }
}
