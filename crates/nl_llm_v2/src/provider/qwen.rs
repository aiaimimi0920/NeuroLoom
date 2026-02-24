use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};

/// Qwen Code 扩展
///
/// portal.qwen.ai 没有公开的 /v1/models 端点，
/// chat.qwen.ai/api/v1/models 需要 session cookie (不接受 OAuth Bearer token)。
/// 使用来自 CLIProxyAPI 参考的静态模型列表。
pub struct QwenExtension;

impl QwenExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProviderExtension for QwenExtension {
    fn id(&self) -> &str {
        "qwen"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "qwen3-coder-plus".to_string(),
                description: "Advanced code generation and understanding model".to_string(),
            },
            ModelInfo {
                id: "qwen3-coder-flash".to_string(),
                description: "Fast code generation model".to_string(),
            },
            ModelInfo {
                id: "coder-model".to_string(),
                description: "Qwen 3.5 Plus — efficient hybrid model (1M context)".to_string(),
            },
            ModelInfo {
                id: "vision-model".to_string(),
                description: "Qwen3 Vision multimodal model".to_string(),
            },
        ])
    }
}
