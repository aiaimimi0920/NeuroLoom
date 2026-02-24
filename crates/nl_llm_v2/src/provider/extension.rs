use async_trait::async_trait;
use crate::auth::traits::Authenticator;

/// 获取到的模型信息
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub description: String, // 模型描述或能力标签
}

/// 扩展 API 接口：各大平台特有的周边能力
#[async_trait]
pub trait ProviderExtension: Send + Sync {
    /// 扩展能力标识
    fn id(&self) -> &str;

    /// 获取可用模型列表
    async fn list_models(
        &self, 
        http: &reqwest::Client, 
        auth: &mut dyn Authenticator
    ) -> anyhow::Result<Vec<ModelInfo>>;
    
    /// (预留) 获取平台的余额或额度信息
    async fn get_balance(
        &self, 
        _http: &reqwest::Client, 
        _auth: &mut dyn Authenticator
    ) -> anyhow::Result<Option<String>> {
        Ok(None) // 默认不实现
    }
}
