use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;

/// Gemini 官方 API 扩展
///
/// 通过 `GET /v1beta/models?key=xxx` 获取远程可用模型列表。
/// API Key 在构造时注入，不依赖 Authenticator 的 downcast。
pub struct GeminiExtension {
    api_key: String,
}

impl GeminiExtension {
    pub fn new() -> Self {
        Self { api_key: String::new() }
    }

    /// 设置 API Key（由 ClientBuilder::with_gemini_api_key 调用）
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = key.into();
        self
    }

    /// 获取当前 API Key 的引用
    pub fn key(&self) -> &str {
        &self.api_key
    }
}

#[async_trait]
impl ProviderExtension for GeminiExtension {
    fn id(&self) -> &str {
        "gemini"
    }

    async fn list_models(
        &self,
        http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("Gemini API Key not set. Use with_gemini_api_key() to configure."));
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            self.api_key
        );

        let res = http.get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch models: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("list_models failed with status {}: {}", status, body));
        }

        let json: serde_json::Value = res.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse models response: {}", e))?;

        let models = json.get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter().filter_map(|m| {
                    let name = m.get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default();
                    // API 返回格式为 "models/gemini-2.5-flash" → 只取 "gemini-2.5-flash"
                    let id = name.strip_prefix("models/").unwrap_or(name).to_string();
                    let description = m.get("displayName")
                        .and_then(|d| d.as_str())
                        .unwrap_or_default()
                        .to_string();

                    // 过滤掉不支持生成的模型（如仅支持 embedding 的模型）
                    let methods = m.get("supportedGenerationMethods")
                        .and_then(|s| s.as_array());
                    let supports_generate = methods.map(|arr| {
                        arr.iter().any(|m| {
                            m.as_str() == Some("generateContent") || m.as_str() == Some("streamGenerateContent")
                        })
                    }).unwrap_or(false);

                    if supports_generate && !id.is_empty() {
                        Some(ModelInfo { id, description })
                    } else {
                        None
                    }
                }).collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(models)
    }

    async fn get_balance(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<String>> {
        // Gemini 官方 API 无额度/余额概念
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Gemini 免费: 15 RPM, 付费: 2,000 RPM
        // 使用保守值 30 作为默认
        ConcurrencyConfig::new(30)
    }
}
