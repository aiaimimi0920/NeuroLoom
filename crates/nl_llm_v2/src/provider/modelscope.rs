use crate::concurrency::ConcurrencyConfig;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::balance::BalanceStatus;
use crate::auth::traits::Authenticator;
use reqwest::Client;
use std::sync::Arc;

/// ModelScope (魔搭) 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api-inference.modelscope.cn/v1";

/// ModelScope (魔搭) 平台扩展
///
/// 阿里巴巴魔搭社区开源模型 API-Inference 服务，兼容 OpenAI 协议。
///
/// ## 认证方式
///
/// 使用 ModelScope Access Token，标准 `Authorization: Bearer <token>` 格式。
/// 获取 Token: <https://modelscope.cn/my/myaccesstoken>
///
/// ## 支持的模型
///
/// ModelScope 使用 `组织/模型名` 格式的 Model-Id，例如：
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `Qwen/Qwen3-235B-A22B` | 128K | Qwen3 旗舰 MoE 模型 |
/// | `Qwen/Qwen2.5-Coder-32B-Instruct` | 128K | Qwen2.5 编程专用 |
/// | `Qwen/QVQ-72B-Preview` | 128K | 视觉推理模型 |
///
/// > **注意**: 可用模型取决于魔搭平台的 API-Inference 部署状态，
/// > 并非所有模型都始终可用。
///
/// ## 并发策略
///
/// - 官方上限: 10 并发
/// - 初始并发: 3
pub struct ModelScopeExtension {
    base_url: String,
}

impl ModelScopeExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for ModelScopeExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn modelscope_models() -> Vec<ModelInfo> {
    vec![
        // === Qwen3 系列 ===
        ModelInfo {
            id: "Qwen/Qwen3-235B-A22B".to_string(),
            description: "Qwen3 旗舰 MoE 模型，128K context，支持思考模式".to_string(),
        },
        ModelInfo {
            id: "Qwen/Qwen3-32B".to_string(),
            description: "Qwen3 32B Dense 模型，128K context".to_string(),
        },
        ModelInfo {
            id: "Qwen/Qwen3-8B".to_string(),
            description: "Qwen3 8B 轻量模型，128K context".to_string(),
        },
        // === Qwen2.5 编程系列 ===
        ModelInfo {
            id: "Qwen/Qwen2.5-Coder-32B-Instruct".to_string(),
            description: "Qwen2.5 Coder 32B — 编程专用，128K context".to_string(),
        },
        // === 视觉模型 ===
        ModelInfo {
            id: "Qwen/QVQ-72B-Preview".to_string(),
            description: "QVQ 72B — 视觉推理模型，128K context".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for ModelScopeExtension {
    fn id(&self) -> &str {
        "modelscope"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // ModelScope 的可用模型随平台部署状态变化，返回静态列表作为参考
        Ok(modelscope_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // ModelScope API-Inference 免费服务，无余额概念
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 10,
            initial_limit: 3,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<ModelScopeExtension> {
    Arc::new(ModelScopeExtension::new())
}
