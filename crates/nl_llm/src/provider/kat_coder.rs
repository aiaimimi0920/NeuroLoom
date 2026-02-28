use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// KAT-Coder 默认 Vanchin 网关基础 URL
const DEFAULT_BASE_URL: &str = "https://vanchin.streamlake.ai/api/gateway/v1/endpoints";

/// KAT-Coder (StreamLake) 平台扩展
///
/// 快手旗下 StreamLake 平台推出的代码大模型服务，通过 Vanchin 网关提供
/// OpenAI 兼容 API。
///
/// ## 核心特性
///
/// - **Vanchin 网关**: 所有请求通过 `vanchin.streamlake.ai` 网关路由
/// - **智能路由**: 通过 API Key 和 Model 名称自动路由到对应的 Endpoint
/// - **代码专精**: 专门针对代码生成和编程辅助场景优化
///
/// ## 模型说明
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `KAT-Coder-Pro` | Chat, Tools, Streaming | 128K | 旗舰代码模型 |
/// | `KAT-Coder-Air-V1` | Chat, Tools, Streaming | 128K | 轻量级代码模型 |
///
/// ## 并发策略
///
/// KAT-Coder 是相对较新的平台，暂无公开的并发限制数据：
/// - 官方最大并发：10（保守估计）
/// - 初始并发：3
/// - 最大探测上限：15
/// - 使用 AIMD 算法动态调节
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("kat_coder")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .with_concurrency()
///     .build();
///
/// // 基础对话
/// let req = nl_llm::PrimitiveRequest::single_user_message("写一个快速排序算法");
/// ```
pub struct KatCoderExtension {
    /// API 基础 URL
    base_url: String,
}

impl KatCoderExtension {
    /// 创建新的 KAT-Coder 扩展
    ///
    /// 默认使用 Vanchin 网关地址。
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL
    ///
    /// 无需修改默认 Vanchin 节点即可连接，主要用于搭建了专属网关或本地测试的场景。
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for KatCoderExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// KAT-Coder 内置模型列表
fn kat_coder_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "kat-coder-pro".to_string(),
            description: "KAT-Coder Pro — 旗舰代码大模型，128K 上下文，支持代码生成与编程辅助"
                .to_string(),
        },
        ModelInfo {
            id: "kat-coder-pro-v1".to_string(),
            description: "KAT-Coder Pro V1 — 旗舰版本 V1，用于 Claude API 代理模式".to_string(),
        },
        ModelInfo {
            id: "kat-coder-air-v1".to_string(),
            description: "KAT-Coder Air V1 — 轻量级代码模型，速度更快、延迟更低".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for KatCoderExtension {
    fn id(&self) -> &str {
        "kat_coder"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // KAT-Coder 目前没有公开的模型列表 API，
        // 直接返回精心维护的静态列表
        Ok(kat_coder_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // StreamLake 平台目前没有公开的余额查询 API
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // KAT-Coder 是相对较新的平台，暂无官方并发限制文档
        // 采用保守策略
        ConcurrencyConfig {
            official_max: 10,
            initial_limit: 3,
            min_limit: 1,
            max_limit: 15,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<KatCoderExtension> {
    Arc::new(KatCoderExtension::new())
}
