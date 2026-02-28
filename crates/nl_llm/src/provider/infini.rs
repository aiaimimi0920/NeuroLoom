use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// 无问芯穹（Infinigence AI）静态模型列表扩展
pub struct InfiniExtension;

impl InfiniExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InfiniExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn infini_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "deepseek-v3.1".to_string(),
            description: "DeepSeek V3.1 — 通用对话与工具调用".to_string(),
        },
        ModelInfo {
            id: "deepseek-r1".to_string(),
            description: "DeepSeek R1 — 推理增强".to_string(),
        },
        ModelInfo {
            id: "qwen3-coder-plus".to_string(),
            description: "Qwen3 Coder Plus — 代码与 Agent 任务".to_string(),
        },
        ModelInfo {
            id: "qwen2.5-vl-72b-instruct".to_string(),
            description: "Qwen2.5 VL 72B — 多模态理解".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for InfiniExtension {
    fn id(&self) -> &str {
        "infini"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(infini_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(32)
    }
}

pub fn extension() -> Arc<InfiniExtension> {
    Arc::new(InfiniExtension::new())
}
