use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;

pub struct IFlowExtension;

#[async_trait]
impl ProviderExtension for IFlowExtension {
    fn id(&self) -> &str {
        "iflow"
    }

    async fn list_models(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get("https://apis.iflow.cn/v1/models");

        // 显式触发认证过程（获取/刷新 API Key）
        auth.refresh().await?;
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("iFlow models API failed ({}): {}", status, err_text));
        }

        let text = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Invalid json response: {}", e))?;

        let mut models = Vec::new();

        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            for m in data {
                if let Some(id) = m.get("id").and_then(|i| i.as_str()) {
                    models.push(ModelInfo {
                        id: id.to_string(),
                        description: "iFlow provider model".to_string(), // 暂时使用简单描述
                    });
                }
            }
        }

        // 以 id 字母排序
        models.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(models)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // iFlow 企业内部服务，并发限制较宽松
        ConcurrencyConfig::new(100)
    }
}
