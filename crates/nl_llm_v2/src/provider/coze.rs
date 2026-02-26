use async_trait::async_trait;
use reqwest::Client;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// Coze API 扩展实现
pub struct CozeExtension {}

impl CozeExtension {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CozeExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderExtension for CozeExtension {
    fn id(&self) -> &str {
        "coze"
    }

    async fn list_models(
        &self,
        _client: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // Coze uses bot_id, yielding a dummy model for structure satisfaction
        Ok(vec![ModelInfo {
            id: "coze-bot-id".to_string(),
            description: "Enter your deployed Bot ID as the model".to_string(),
        }])
    }

    async fn get_balance(
        &self,
        _client: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // Not cleanly supported
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::default()
    }
}
