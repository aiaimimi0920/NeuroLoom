use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use super::extension::{ProviderExtension, ModelInfo};
use super::balance::BalanceStatus;
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

pub struct GeminiCliExtension;

#[async_trait]
impl ProviderExtension for GeminiCliExtension {
    fn id(&self) -> &str {
        "gemini_cli"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // 由于 Gemini CLI 的 OAuth Client ID 对 v1internal:fetchAvailableModels 存在鉴权拦截 (403)，
        // 此处返回框架内置的已知静态模型列表
        
        // 静态支持的模型列表预设
        let models = vec![
            ("gemini-2.5-pro", "Gemini 2.5 Pro"),
            ("gemini-2.5-flash", "Gemini 2.5 Flash"),
            ("gemini-2.0-flash", "Gemini 2.0 Flash"),
            ("gemini-2.0-pro-exp-02-05", "Gemini 2.0 Pro Exp (02-05)"),
            ("gemini-2.0-flash-thinking-exp-01-21", "Gemini 2.0 Flash Thinking Exp (01-21)"),
            ("gemini-1.5-pro", "Gemini 1.5 Pro"),
            ("gemini-1.5-flash", "Gemini 1.5 Flash"),
            ("gemini-1.5-pro-002", "Gemini 1.5 Pro (002)"),
            ("gemini-1.5-flash-002", "Gemini 1.5 Flash (002)"),
        ];

        let available_models = models.into_iter()
            .map(|(id, desc)| ModelInfo {
                id: id.to_string(),
                description: desc.to_string(),
            })
            .collect();

        Ok(available_models)
    }

    async fn get_balance(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // Gemini CLI 通过 CloudCode PA 使用免费配额，无独立的额度查询 API
        // loadCodeAssist 仅用于获取 project_id（已在 Auth 层处理），不含额度信息
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // Gemini CLI: 使用保守的并发限制
        ConcurrencyConfig::new(10)
    }
}

pub fn extension() -> Arc<GeminiCliExtension> {
    Arc::new(GeminiCliExtension)
}
