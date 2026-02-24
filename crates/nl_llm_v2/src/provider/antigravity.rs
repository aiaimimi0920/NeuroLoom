use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};
use serde_json::Value;

pub struct AntigravityExtension;

#[async_trait]
impl ProviderExtension for AntigravityExtension {
    fn id(&self) -> &str {
        "antigravity"
    }

    async fn list_models(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // 请求配置
        let url = "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels";
        let body = serde_json::json!({});

        // 获取并刷新凭据
        auth.refresh().await?;
        
        // 由于是从 extension 调用，不再通过 Pipeline，注入必要的 Agent
        let req = http.post(url)
            .header("User-Agent", "antigravity/1.104.0 darwin/arm64")
            .json(&body);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Antigravity models API failed ({}): {}", status, err_text));
        }

        let text = resp.text().await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Invalid json response: {}", e))?;

        let mut available_models = Vec::new();

        // 提取 models 字段
        if let Some(models) = json.get("models").and_then(|m| m.as_object()) {
            for (id, model_data) in models {
                // 内部模型过滤
                let skip_models = ["chat_20706", "chat_23310", "gemini-2.5-flash-thinking", "gemini-3-pro-low"];
                if skip_models.contains(&id.as_str()) {
                    continue;
                }

                // 解析能力描述
                let mut caps = Vec::new();
                
                // 模型基本描述
                if let Some(desc) = model_data.get("description").and_then(|v| v.as_str()) {
                    caps.push(desc.to_string());
                }
                
                // 是否推荐
                if let Some(is_rec) = model_data.get("isRecommended").and_then(|v| v.as_bool()) {
                    if is_rec { caps.push("Recommended".to_string()); }
                }

                // fallback 到静态字典描述
                let static_desc = match id.as_str() {
                    "gemini-2.5-pro" => "Gemini 2.5 Pro — 最强推理 (1M token)",
                    "gemini-2.5-flash" => "Gemini 2.5 Flash — 快速多模态 (1M token)",
                    "gemini-2.5-flash-lite" => "Gemini 2.5 Flash Lite — 最低成本",
                    "gemini-3-pro-high" => "Gemini 3 Pro",
                    "gemini-3.1-pro-high" => "Gemini 3.1 Pro (最新)",
                    "gemini-3-flash" => "Gemini 3 Flash",
                    "claude-sonnet-4-6" => "Claude Sonnet 4.6 — 200K ctx",
                    "claude-opus-4-6-thinking" => "Claude Opus 4.6 + Thinking — 1M ctx",
                    _ => ""
                };

                if caps.is_empty() && !static_desc.is_empty() {
                    caps.push(static_desc.to_string());
                }
                
                available_models.push(ModelInfo {
                    id: id.to_string(),
                    description: caps.join(" | "),
                });
            }
        }

        Ok(available_models)
    }

    async fn get_balance(
        &self, 
        http: &reqwest::Client, 
        auth: &mut dyn Authenticator
    ) -> anyhow::Result<Option<String>> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
        let body = serde_json::json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        auth.refresh().await?;

        let req = http.post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-python/3.12.0")
            .header("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
            .json(&body);
        
        let req = auth.inject(req)?;
        let res = req.send().await?;
        let status = res.status();
        let text = res.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("loadCodeAssist failed ({}): {}", status, text));
        }

        let json: Value = serde_json::from_str(&text)?;
        let mut result = String::new();

        if let Some(project) = json.get("cloudaicompanionProject") {
            if let Some(p) = project.as_str() {
                result.push_str(&format!("项目 ID: {}\n", p));
            } else if let Some(obj) = project.as_object() {
                if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                    result.push_str(&format!("项目 ID: {}\n", id));
                }
            }
        }

        if let Some(tier) = json.get("currentTier") {
            let id = tier.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            let name = tier.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            result.push_str(&format!("当前 Tier: {} ({})\n", name, id));
        }

        if let Some(paid) = json.get("paidTier") {
            let name = paid.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            if let Some(credits) = paid.get("availableCredits").and_then(|v| v.as_array()) {
                for c in credits {
                    let amount = c.get("creditAmount").and_then(|v| v.as_str()).unwrap_or("?");
                    let ctype = c.get("creditType").and_then(|v| v.as_str()).unwrap_or("?");
                    result.push_str(&format!("付费 Tier: {} (额度: {} {})\n", name, amount, ctype));
                }
            }
        }

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}
