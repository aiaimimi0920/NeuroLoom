use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// 七牛云 AI 推理模型解析器
///
/// 七牛云大模型服务是一个基于兼容 OpenAI 的 API 层，
/// 支持调用主流模型（例如 Qwen 系列）。
/// 默认视其支持主流的文本聊天、流式输出、视觉和工具调用功能。
pub struct QiniuModelResolver;

impl QiniuModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for QiniuModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for QiniuModelResolver {
    fn resolve(&self, model: &str) -> String {
        model.to_string()
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // 作为代理服务，允许透传基础的全栈能力
        capability.contains(Capability::CHAT) 
            || capability.contains(Capability::STREAMING) 
            || capability.contains(Capability::VISION) 
            || capability.contains(Capability::TOOLS)
    }

    fn max_context(&self, model: &str) -> usize {
        // 大多以开源模型或千问为主，基于当前趋势给予通用宽松界限
        if model.contains("qwen") {
            32_000
        } else {
            8192
        }
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max, 0)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

/// 兼容 OpenAI `/v1/models` 的响应结构
#[derive(Debug, Deserialize)]
struct QiniuModelsResponse {
    #[serde(default)]
    data: Vec<QiniuModelItem>,
}

#[derive(Debug, Deserialize)]
struct QiniuModelItem {
    id: String,
}

/// 七牛云 AI 推理扩展：动态查询并返回支持的模型
pub struct QiniuExtension;

impl QiniuExtension {
    pub fn new() -> Self {
        Self {}
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "qwen-plus".to_string(),
                description: "Fallback model (Qwen Plus series)".to_string(),
            },
            ModelInfo {
                id: "qwen-max".to_string(),
                description: "Fallback model (Qwen Max series)".to_string(),
            },
            ModelInfo {
                id: "qwen-turbo".to_string(),
                description: "Fallback model (Qwen Turbo series)".to_string(),
            }
        ]
    }
}

impl Default for QiniuExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderExtension for QiniuExtension {
    fn id(&self) -> &str {
        "qiniu"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get("https://ai.qiniuapi.com/v1/models");
        let req = auth.inject(req)?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let payload: QiniuModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| anyhow::anyhow!("qiniu /models 解析失败: {}", e))?;

                if payload.data.is_empty() {
                    Ok(Self::fallback_models())
                } else {
                    Ok(payload
                        .data
                        .into_iter()
                        .map(|m| ModelInfo {
                            id: m.id,
                            description: String::new(),
                        })
                        .collect())
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eprintln!(
                    "[qiniu] /models 请求失败 ({}): {}，使用静态兜底列表",
                    status, body
                );
                Ok(Self::fallback_models())
            }
            Err(e) => {
                eprintln!("[qiniu] /models 网络错误: {}，使用静态兜底列表", e);
                Ok(Self::fallback_models())
            }
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(20)
    }
}
