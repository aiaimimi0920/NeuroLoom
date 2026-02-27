use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::ocoolai::mainstream_models;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// ocoolAI 平台扩展
///
/// 提供 ocoolAI 平台特定的功能实现。
pub struct OcoolAiExtension {
    base_url: String,
}

impl OcoolAiExtension {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.ocoolai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Default for OcoolAiExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// ocoolAI 热门模型列表
///
/// ocoolAI 平台的 /models 端点返回的数据可能不完整，
/// 因此使用静态列表维护主流模型。数据来源：平台官网 2026-02。
fn ocoolai_models() -> Vec<ModelInfo> {
    mainstream_models()
        .iter()
        .map(|(id, description)| ModelInfo {
            id: (*id).to_string(),
            description: (*description).to_string(),
        })
        .collect()
}

#[async_trait]
impl ProviderExtension for OcoolAiExtension {
    fn id(&self) -> &str {
        "ocoolai"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // ocoolAI 的 /models 端点可能不完整，使用静态列表
        Ok(ocoolai_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // ocoolAI 作为中转平台，使用保守的并发配置
        // 具体限制取决于用户账户等级
        ConcurrencyConfig::new(50)
    }
}

/// 返回 Arc 包装好的扩展实例（供 preset 使用）
pub fn extension() -> Arc<OcoolAiExtension> {
    Arc::new(OcoolAiExtension::new())
}
