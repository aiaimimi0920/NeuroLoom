use std::sync::Arc;

use crate::auth::providers::{ApiKeyAuth, MultiKeyAuth, MultiKeyMode};
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::aionly::{AiOnlyExtension, AiOnlyModelResolver};
use crate::site::base::openai::OpenAiSite;

pub fn builder() -> ClientBuilder {
    let base_url = std::env::var("AIONLY_BASE_URL")
        .unwrap_or_else(|_| "https://api.aiionly.com/v1".to_string());

    let mut builder = ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(AiOnlyModelResolver::new())
        .with_extension(Arc::new(AiOnlyExtension::new(base_url)))
        .default_model("gpt-4o");

    if let Some(multi_key_auth) = read_multi_key_auth_from_env() {
        builder = builder.auth(multi_key_auth);
    }

    builder
}

fn read_multi_key_auth_from_env() -> Option<MultiKeyAuth> {
    let raw = std::env::var("AIONLY_API_KEYS").ok().or_else(|| std::env::var("AIONLY_API_KEY").ok())?;
    let keys: Vec<String> = raw
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();

    if keys.is_empty() {
        return None;
    }

    let mode = match std::env::var("AIONLY_MULTI_KEY_MODE")
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
    pub fn build_aionly(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(AiOnlyExtension::new(&base_url)))
            .build()
    }

    pub fn build_aionly_multi(
        base_url: impl Into<String>,
        api_keys: Vec<String>,
        mode: MultiKeyMode,
    ) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(MultiKeyAuth::new(api_keys).with_mode(mode))
            .with_extension(Arc::new(AiOnlyExtension::new(&base_url)))
            .build()
    }
}
