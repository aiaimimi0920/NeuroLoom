use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// GPUStack 模型解析器
///
/// GPUStack 作为一个本地/私有化部署的大模型推理集群代理，
/// 可以挂载任意开源或专用模型，提供标准的 OpenAI 兼容接口。
/// 默认认为其支持基本的文本聊天和流式输出能力。
pub struct GpuStackModelResolver;

impl GpuStackModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GpuStackModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for GpuStackModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 对于 GPUStack，保留原样运行的模型名
        model.to_string()
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // 作为自部署大模型集群代理，支持全栈能力，全部放行以确保能充分利用后端挂载的模型
        let supported =
            Capability::CHAT | Capability::STREAMING | Capability::VISION | Capability::TOOLS;
        supported.contains(capability)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 本地服务上下文大多取决于 VRAM 限制及部署配置，此处提供一个通用的宽松界限
        128_000
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (100_000, 28_000)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        // 智能水平视集群部署的具体模型而定，设为通用水平
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

/// 兼容 OpenAI `/v1/models` 的响应结构
#[derive(Debug, Deserialize)]
struct GpuStackModelsResponse {
    #[serde(default)]
    data: Vec<GpuStackModelItem>,
}

#[derive(Debug, Deserialize)]
struct GpuStackModelItem {
    id: String,
}

/// GPUStack 扩展：动态从服务端获取运行中的活跃模型列表
pub struct GpuStackExtension {
    base_url: String,
}

impl GpuStackExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn models_url(&self) -> String {
        if self.base_url.ends_with("/v1") {
            format!("{}/models", self.base_url)
        } else {
            format!("{}/v1/models", self.base_url)
        }
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "llama3".to_string(),
                description: "Fallback model (Local GGUF/Safetensors)".to_string(),
            },
            ModelInfo {
                id: "qwen2".to_string(),
                description: "Fallback model (Local GGUF/Safetensors)".to_string(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{GpuStackExtension, GpuStackModelResolver};
    use crate::model::{Capability, ModelResolver};

    #[test]
    fn models_url_handles_v1_base() {
        let ext = GpuStackExtension::new("http://127.0.0.1:8080/v1");
        assert_eq!(ext.models_url(), "http://127.0.0.1:8080/v1/models");
    }

    #[test]
    fn models_url_patches_plain_base() {
        let ext = GpuStackExtension::new("http://127.0.0.1:8080");
        assert_eq!(ext.models_url(), "http://127.0.0.1:8080/v1/models");
    }

    #[test]
    fn capability_check_requires_all_requested_flags() {
        let resolver = GpuStackModelResolver::new();
        assert!(resolver.has_capability("llama3", Capability::CHAT));
        assert!(resolver.has_capability("llama3", Capability::CHAT | Capability::STREAMING));
        assert!(!resolver.has_capability("llama3", Capability::CHAT | Capability::THINKING));
    }
}

#[async_trait]
impl ProviderExtension for GpuStackExtension {
    fn id(&self) -> &str {
        "gpustack"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = http.get(self.models_url());
        let req = auth.inject(req)?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let payload: GpuStackModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| anyhow::anyhow!("gpustack /models 解析失败: {}", e))?;

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
                    "[gpustack] /models 请求失败 ({}): {}，使用静态兜底列表",
                    status, body
                );
                Ok(Self::fallback_models())
            }
            Err(e) => {
                eprintln!("[gpustack] /models 网络错误: {}，使用静态兜底列表", e);
                Ok(Self::fallback_models())
            }
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 自治集群环境，默认并发保守一点防止单挂节点宕机
        ConcurrencyConfig::new(20)
    }
}
