use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};

/// Vertex AI 扩展
///
/// 通过 Vertex AI Discovery API 获取可用模型列表。
/// 需要 project_id 和 location 来构建 API URL。
pub struct VertexExtension {
    #[allow(dead_code)]
    project_id: String,
    location: String,
}

impl VertexExtension {
    pub fn new(project_id: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            location: location.into(),
        }
    }
}

#[async_trait]
impl ProviderExtension for VertexExtension {
    fn id(&self) -> &str {
        "vertex"
    }

    async fn list_models(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // 确保 Token 有效
        if auth.needs_refresh() {
            auth.refresh().await?;
        }

        // Vertex AI 模型列表端点 (Model Garden)
        // 注意：使用 v1beta1 版本，v1 不支持 publisher models 端点
        let url = format!(
            "https://{}-aiplatform.googleapis.com/v1beta1/publishers/google/models",
            self.location
        );

        let req = http.get(&url)
            .header("Content-Type", "application/json");

        // 注入 Bearer Token
        let req = auth.inject(req)?;

        let res = req.send().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch models: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("list_models failed ({}): {}", status, body));
        }

        let json: serde_json::Value = res.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse models response: {}", e))?;

        let models = json.get("models")
            .or_else(|| json.get("publisherModels"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter().filter_map(|m| {
                    // Vertex API 返回格式可能是:
                    // "name": "publishers/google/models/gemini-2.5-flash"
                    let name = m.get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default();

                    // 提取模型短名
                    let id = name.rsplit('/').next().unwrap_or(name).to_string();

                    let description = m.get("displayName")
                        .or_else(|| m.get("openSourceCategory"))
                        .and_then(|d| d.as_str())
                        .unwrap_or_default()
                        .to_string();

                    // 过滤掉空 ID
                    if !id.is_empty() {
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
        Ok(None)
    }
}
