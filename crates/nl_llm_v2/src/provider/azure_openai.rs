use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// Azure OpenAI 扩展
///
/// 微软 Azure 云平台上的 OpenAI 模型服务。
/// 提供企业级 SLA、数据隔离和合规保障。
///
/// ## 与 OpenAI 直连的区别
///
/// | 特性 | OpenAI | Azure OpenAI |
/// |------|--------|--------------|
/// | URL | `api.openai.com/v1` | `{resource}.openai.azure.com/openai/deployments/{name}` |
/// | 认证 | `Authorization: Bearer` | `api-key` 请求头 |
/// | 模型 | 模型名（如 `gpt-4o`） | 部署名（由用户自定义） |
/// | API 版本 | 无需指定 | `?api-version=2024-12-01-preview` |
///
/// ## 申请流程
///
/// 1. 注册 Azure 账号（有 $200 免费试用额度）
/// 2. 在 portal.azure.com 创建 "Azure OpenAI" 资源
/// 3. 在 Azure OpenAI Studio 中创建模型部署（deployment）
/// 4. 获取 endpoint URL 和 API key
///
/// ## 支持的模型（取决于用户部署）
///
/// | 可部署模型 | 说明 |
/// |-----------|------|
/// | `gpt-4o` | GPT-4o，128K context |
/// | `gpt-4o-mini` | GPT-4o Mini，128K context |
/// | `gpt-4.1` | GPT-4.1，1M context |
/// | `gpt-4.1-mini` | GPT-4.1 Mini，1M context |
/// | `o3-mini` | o3-mini 推理模型 |
///
/// > **注意**: 实际可用模型取决于用户部署和地区
///
/// ## 并发策略
///
/// - 默认限制: 10 并发
/// - 初始并发: 3
pub struct AzureOpenAiExtension;

impl AzureOpenAiExtension {
    pub fn new() -> Self { Self }
}

impl Default for AzureOpenAiExtension {
    fn default() -> Self { Self::new() }
}

fn azure_openai_models() -> Vec<ModelInfo> {
    vec![
        // 这些是 Azure 上常见的可部署模型
        // 实际可用的模型取决于用户的 deployment
        ModelInfo { id: "gpt-4o".to_string(), description: "GPT-4o，128K context".to_string() },
        ModelInfo { id: "gpt-4o-mini".to_string(), description: "GPT-4o Mini，128K context".to_string() },
        ModelInfo { id: "gpt-4.1".to_string(), description: "GPT-4.1，1M context".to_string() },
        ModelInfo { id: "gpt-4.1-mini".to_string(), description: "GPT-4.1 Mini，1M context".to_string() },
        ModelInfo { id: "o3-mini".to_string(), description: "o3-mini 推理模型".to_string() },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for AzureOpenAiExtension {
    fn id(&self) -> &str { "azure_openai" }

    async fn list_models(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(azure_openai_models())
    }

    async fn get_balance(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        // Azure 通过订阅计费，无直接余额 API
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig { official_max: 10, initial_limit: 3, ..Default::default() }
    }
}

pub fn extension() -> Arc<AzureOpenAiExtension> {
    Arc::new(AzureOpenAiExtension::new())
}
