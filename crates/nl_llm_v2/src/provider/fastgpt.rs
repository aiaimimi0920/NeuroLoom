use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

use crate::auth::traits::Authenticator;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};

/// FastGPT 扩展定义
///
/// FastGPT 是一个基于 LLM 大语言模型的知识库问答系统。
/// 它提供了与 OpenAI 兼容的 API 接口，因此可以直接复用 OpenAI 的协议处理逻辑。
pub struct FastGptExtension {}

impl Default for FastGptExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl FastGptExtension {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ProviderExtension for FastGptExtension {
    fn id(&self) -> &str {
        "fastgpt"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // FastGPT 主要基于 AppId(通过 API Key 绑定) 驱动对话，
        // 通常不需要像通用模型供应商那样选择模型。
        // 这里提供一个示意性的默认模型，或者可以直接返回空。
        Ok(vec![ModelInfo {
            id: "fastgpt-default".to_string(),
            description: "FastGPT 绑定的默认应用模型".to_string(),
        }])
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // FastGPT 暂未标准化统一的余额查询接口
        Ok(None)
    }
}

pub fn extension() -> Arc<FastGptExtension> {
    Arc::new(FastGptExtension::new())
}
