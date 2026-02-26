use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use reqwest::Client;
use std::sync::Arc;

/// SiliconFlow 默认基础 URL
const DEFAULT_BASE_URL: &str = "https://api.siliconflow.cn/v1";

/// SiliconFlow (硅基流动) 平台扩展
///
/// SiliconFlow 是国内领先的 AI 推理平台，支持多种开源和商业模型，
/// 兼容 OpenAI 协议。模型命名使用 `组织/模型名` 格式。
///
/// ## 认证方式
///
/// 标准 `Authorization: Bearer <key>` 格式。
///
/// ## 模型命名规则
///
/// SiliconFlow 使用分层命名：
/// - **Pro/** 前缀: 高性能推理（如 `Pro/moonshotai/Kimi-K2.5`）
/// - **Free/** 前缀: 免费推理层
/// - **直接模型名**: 标准推理
///
/// ## 支持的模型（示例）
///
/// | 模型 ID | 说明 |
/// |---------|------|
/// | `Pro/moonshotai/Kimi-K2.5` | Kimi K2.5 Pro 推理 |
/// | `Qwen/Qwen3-8B` | Qwen3 8B |
/// | `deepseek-ai/DeepSeek-V3` | DeepSeek V3 |
/// | `deepseek-ai/DeepSeek-R1` | DeepSeek R1 推理 |
///
/// ## 并发策略
///
/// - 官方上限: 20 并发
/// - 初始并发: 5
pub struct SiliconFlowExtension {
    base_url: String,
}

impl SiliconFlowExtension {
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

impl Default for SiliconFlowExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn siliconflow_models() -> Vec<ModelInfo> {
    vec![
        // === Pro 推理层 ===
        ModelInfo {
            id: "Pro/moonshotai/Kimi-K2.5".to_string(),
            description: "Kimi K2.5 — Pro 推理，MoE 旗舰".to_string(),
        },
        ModelInfo {
            id: "Pro/deepseek-ai/DeepSeek-R1".to_string(),
            description: "DeepSeek R1 — Pro 推理，支持思考链".to_string(),
        },
        ModelInfo {
            id: "Pro/deepseek-ai/DeepSeek-V3".to_string(),
            description: "DeepSeek V3 — Pro 推理".to_string(),
        },
        // === 标准推理层 ===
        ModelInfo {
            id: "Qwen/Qwen3-8B".to_string(),
            description: "Qwen3 8B — 轻量模型，支持思考链".to_string(),
        },
        ModelInfo {
            id: "Qwen/Qwen3-32B".to_string(),
            description: "Qwen3 32B — 中等规模，支持思考链".to_string(),
        },
        ModelInfo {
            id: "deepseek-ai/DeepSeek-V3".to_string(),
            description: "DeepSeek V3 — 标准推理".to_string(),
        },
        ModelInfo {
            id: "deepseek-ai/DeepSeek-R1".to_string(),
            description: "DeepSeek R1 — 推理模型，支持思考链".to_string(),
        },
        // === 多模态模型 ===
        ModelInfo {
            id: "Qwen/Qwen3-VL-72B-Instruct".to_string(),
            description: "Qwen3 VL 72B — 多模态，支持视觉/视频，支持思考链".to_string(),
        },
        ModelInfo {
            id: "Qwen/Qwen3-Omni-30B-A3B-Instruct".to_string(),
            description: "Qwen3 Omni 30B — 全模态，支持视觉/音频/视频，支持思考链".to_string(),
        },
        ModelInfo {
            id: "Qwen/Qwen2.5-VL-72B-Instruct".to_string(),
            description: "Qwen2.5 VL 72B — 多模态，支持视觉".to_string(),
        },
        ModelInfo {
            id: "deepseek-ai/deepseek-vl2".to_string(),
            description: "DeepSeek VL2 — 多模态，支持视觉".to_string(),
        },
    ]
}

#[async_trait::async_trait]
impl ProviderExtension for SiliconFlowExtension {
    fn id(&self) -> &str {
        "siliconflow"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(siliconflow_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig {
            official_max: 20,
            initial_limit: 5,
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<SiliconFlowExtension> {
    Arc::new(SiliconFlowExtension::new())
}
