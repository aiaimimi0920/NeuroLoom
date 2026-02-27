use crate::provider::ProviderExtension;
use async_trait::async_trait;

/// 百川智能提供商特定扩展
pub struct BaichuanExtension {
    // 可以在这里持有 reqwest 客户端和认证信息等
}

impl BaichuanExtension {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BaichuanExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderExtension for BaichuanExtension {
    fn id(&self) -> &str {
        "baichuan"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn crate::auth::traits::Authenticator,
    ) -> anyhow::Result<Vec<crate::provider::ModelInfo>> {
        // 百川未提供标准的拉取所有模型接口，或者我们目前不需要动态检测
        // 直接返回一个空实现，或抛出 Unsupported
        Ok(vec![])
    }
}
