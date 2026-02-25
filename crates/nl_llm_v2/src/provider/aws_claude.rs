use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// AWS Claude (Amazon Bedrock) 扩展
///
/// Amazon Bedrock 提供 Claude、Llama 等模型的托管服务。
///
/// ## 认证模式
///
/// ### 1. AK/SK 模式
/// ```text
/// AWS_ACCESS_KEY_ID=AKIA...
/// AWS_SECRET_ACCESS_KEY=xxxxx
/// AWS_REGION=us-east-1
/// ```
/// 需要 AWS SigV4 签名，每个请求动态计算。
///
/// ### 2. API Key 模式
/// ```text
/// AWS_BEDROCK_API_KEY=xxx
/// ```
/// 简单的 Bearer Token 认证。
///
/// ## 支持的 Claude 模型
///
/// | 模型 ID | 说明 | 输入/输出价格 |
/// |---------|------|-------------|
/// | `anthropic.claude-sonnet-4-6-20250514-v1:0` | Claude Sonnet 4.6 | $3/$15 |
/// | `anthropic.claude-opus-4-6-20250514-v1:0` | Claude Opus 4.6 | $15/$75 |
/// | `anthropic.claude-sonnet-4-5-20250929-v1:0` | Claude Sonnet 4.5 | $3/$15 |
/// | `anthropic.claude-opus-4-5-20250915-v1:0` | Claude Opus 4.5 | $15/$75 |
/// | `anthropic.claude-3-5-sonnet-20241022-v2:0` | Claude 3.5 Sonnet v2 | $3/$15 |
/// | `anthropic.claude-3-5-haiku-20241022-v1:0` | Claude 3.5 Haiku | $0.8/$4 |
/// | `anthropic.claude-3-haiku-20240307-v1:0` | Claude 3 Haiku | $0.25/$1.25 |
///
/// ## 并发策略
///
/// - 默认: 10 并发
/// - 初始: 3
pub struct AwsClaudeExtension {
    region: String,
}

impl AwsClaudeExtension {
    pub fn new() -> Self {
        Self { region: "us-east-1".to_string() }
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }
}

impl Default for AwsClaudeExtension {
    fn default() -> Self { Self::new() }
}

fn aws_claude_models() -> Vec<ModelInfo> {
    vec![
        // === Claude 4.6 系列 ===
        ModelInfo { id: "anthropic.claude-sonnet-4-6-20250514-v1:0".to_string(), description: "Claude Sonnet 4.6 (Bedrock)，200K context，$3/$15".to_string() },
        ModelInfo { id: "anthropic.claude-opus-4-6-20250514-v1:0".to_string(), description: "Claude Opus 4.6 (Bedrock)，200K context，$15/$75".to_string() },
        // === Claude 4.5 系列 ===
        ModelInfo { id: "anthropic.claude-sonnet-4-5-20250929-v1:0".to_string(), description: "Claude Sonnet 4.5 (Bedrock)，200K context，$3/$15".to_string() },
        ModelInfo { id: "anthropic.claude-opus-4-5-20250915-v1:0".to_string(), description: "Claude Opus 4.5 (Bedrock)，200K context，$15/$75".to_string() },
        // === Claude 3.5 系列 ===
        ModelInfo { id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(), description: "Claude 3.5 Sonnet v2 (Bedrock)，200K context，$3/$15".to_string() },
        ModelInfo { id: "anthropic.claude-3-5-haiku-20241022-v1:0".to_string(), description: "Claude 3.5 Haiku (Bedrock)，200K context，$0.8/$4".to_string() },
        // === Claude 3 系列 ===
        ModelInfo { id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(), description: "Claude 3 Haiku (Bedrock)，200K context，$0.25/$1.25".to_string() },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for AwsClaudeExtension {
    fn id(&self) -> &str { "aws_claude" }

    async fn list_models(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(aws_claude_models())
    }

    async fn get_balance(&self, _http: &Client, _auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        // AWS 通过订阅计费，无直接余额 API
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig { official_max: 10, initial_limit: 3, ..Default::default() }
    }
}

pub fn extension() -> Arc<AwsClaudeExtension> {
    Arc::new(AwsClaudeExtension::new())
}
