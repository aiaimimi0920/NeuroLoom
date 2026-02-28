use crate::auth::providers::{ApiKeyAuth, MultiKeyAuth, MultiKeyMode};
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::newapi::{NewApiExtension, NewApiModelResolver};
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// NewAPI 预设
///
/// NewAPI 是一种兼容 OpenAI 格式的中转代理服务，通常运行在用户自定义地址。
/// 本预设可以连接到任何 NewAPI 兼容端点（如 Cherry Studio）。
///
/// 使用前请确保：
/// 1. 提供了自定义的 `base_url`（如 `"http://127.0.0.1:3000/v1"`）。
///    如果不通过 Builder 手动覆盖，默认会尝试读取 `NEWAPI_BASE_URL`，
///    若仍为空则回退到 `http://127.0.0.1:3000/v1`。
/// 2. 提供认证信息：单 Key 用 `auth(Auth::api_key("sk-xxx"))`，
///    多 Key 可用 `NEWAPI_API_KEYS=sk-a,sk-b` + `NEWAPI_MULTI_KEY_MODE=random|round_robin`。
pub fn builder() -> ClientBuilder {
    let base_url = read_newapi_base_url();

    let mut builder = ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(NewApiModelResolver::new())
        .with_extension(Arc::new(NewApiExtension::new(&base_url)));

    if let Some(multi_key_auth) = read_multi_key_auth_from_env() {
        builder = builder.auth(multi_key_auth);
    }

    builder
}

fn read_newapi_base_url() -> String {
    std::env::var("NEWAPI_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:3000/v1".to_string())
}

fn read_multi_key_auth_from_env() -> Option<MultiKeyAuth> {
    let raw = std::env::var("NEWAPI_API_KEYS").ok()?;
    let keys: Vec<String> = raw
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();

    if keys.is_empty() {
        return None;
    }

    let mode = match std::env::var("NEWAPI_MULTI_KEY_MODE")
        .unwrap_or_else(|_| "round_robin".to_string())
        .to_lowercase()
        .as_str()
    {
        "random" => MultiKeyMode::Random,
        _ => MultiKeyMode::RoundRobin,
    };

    Some(MultiKeyAuth::new(keys).with_mode(mode))
}

/// 便捷构建器
impl LlmClient {
    pub fn build_newapi(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(NewApiExtension::new(&base_url)))
            .build()
    }

    pub fn build_newapi_multi(
        base_url: impl Into<String>,
        api_keys: Vec<String>,
        mode: MultiKeyMode,
    ) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(MultiKeyAuth::new(api_keys).with_mode(mode))
            .with_extension(Arc::new(NewApiExtension::new(&base_url)))
            .build()
    }
}
