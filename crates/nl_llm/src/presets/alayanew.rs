use std::sync::Arc;

use crate::auth::providers::{ApiKeyAuth, MultiKeyAuth, MultiKeyMode};
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::alayanew::{AlayanewExtension, AlayanewModelResolver};
use crate::site::base::openai::OpenAiSite;

const DEFAULT_ALAYANEW_BASE_URL: &str = "https://deepseek.alayanew.com/v1";

pub fn builder() -> ClientBuilder {
    let base_url = read_env_first(&["ALAYANEW_BASE_URL", "ALAYA_NEW_BASE_URL"])
        .unwrap_or_else(|| DEFAULT_ALAYANEW_BASE_URL.to_string());

    let mut builder = ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(AlayanewModelResolver::new())
        .with_extension(Arc::new(AlayanewExtension::new(&base_url)))
        .default_model("deepseek-chat");

    if let Some(multi_key_auth) = read_multi_key_auth_from_env() {
        builder = builder.auth(multi_key_auth);
    } else if let Some(api_key) = read_env_first(&["ALAYANEW_API_KEY", "ALAYA_NEW_API_KEY"]) {
        builder = builder.auth(ApiKeyAuth::new(api_key));
    }

    builder
}

fn read_multi_key_auth_from_env() -> Option<MultiKeyAuth> {
    let raw = read_env_first(&[
        "ALAYANEW_API_KEYS",
        "ALAYA_NEW_API_KEYS",
        "ALAYANEW_API_KEY",
        "ALAYA_NEW_API_KEY",
    ])?;
    let keys: Vec<String> = raw
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();

    if keys.is_empty() {
        return None;
    }

    let mode = match read_env_first(&["ALAYANEW_MULTI_KEY_MODE", "ALAYA_NEW_MULTI_KEY_MODE"])
        .unwrap_or_else(|| "round_robin".to_string())
        .to_lowercase()
        .as_str()
    {
        "random" => MultiKeyMode::Random,
        _ => MultiKeyMode::RoundRobin,
    };

    Some(MultiKeyAuth::new(keys).with_mode(mode))
}

fn read_env_first(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl LlmClient {
    pub fn build_alayanew(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(AlayanewExtension::new(&base_url)))
            .build()
    }

    pub fn build_alayanew_multi(
        base_url: impl Into<String>,
        api_keys: Vec<String>,
        mode: MultiKeyMode,
    ) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(MultiKeyAuth::new(api_keys).with_mode(mode))
            .with_extension(Arc::new(AlayanewExtension::new(&base_url)))
            .build()
    }
}
