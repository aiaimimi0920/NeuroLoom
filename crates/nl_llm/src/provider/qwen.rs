use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// Qwen (通义千问) 默认 API 基础 URL
const DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

/// Qwen (通义千问) 平台扩展
///
/// 阿里云百炼平台提供的 DashScope 兼容 API，支持 Qwen 系列模型。
///
/// ## 核心特性
///
/// - **静态模型列表**: API 返回的模型过多且包含旧版，使用严格筛选的静态优质模型列表
/// - **并发控制**: 阿里云 API 针对不同层级有不同的并发限制
/// - **模型丰富**: 支持通用大模型、代码模型、视觉模型、推理模型
///
/// ## 模型说明
///
/// | 类别 | 模型 | 说明 |
/// |------|------|------|
/// | 通用大模型 | qwen-max, qwen-plus, qwen-turbo | 全能型模型 |
/// | 代码模型 | qwen2.5-coder-32b-instruct | 顶级开源代码模型 |
/// | 视觉模型 | qwen-vl-max, qwen-vl-plus | 多模态理解 |
/// | 推理模型 | qwq-plus, qwq-32b-preview | 链式思考 |
///
/// ## 并发策略
///
/// 采用保守的并发配置：
/// - 官方最大并发：20（保守估计）
/// - 初始并发：5（避免触发限流）
/// - 最大探测上限：30（AIMD 算法可探测到的上限）
/// - 使用 AIMD 算法动态调节
///
/// ## 示例
///
/// ```rust,no_run
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("qwen")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .with_concurrency()
///     .build();
///
/// // 使用代码模型
/// let req = nl_llm::PrimitiveRequest::single_user_message("Write a hello world")
///     .with_model("coder");  // 自动解析为 qwen2.5-coder-32b-instruct
/// ```
pub struct QwenExtension {
    /// API 基础 URL
    base_url: String,
}

impl QwenExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL（用于代理场景）
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }
}

impl Default for QwenExtension {
    fn default() -> Self {
        Self::new()
    }
}

/// 筛选后的优质 Qwen 模型列表
fn qwen_models() -> Vec<ModelInfo> {
    vec![
        // === 主力大模型 ===
        ModelInfo {
            id: "qwen-max".to_string(),
            description: "Qwen Max — 通义千问千亿级别超大规模模型，综合能力最强".to_string(),
        },
        ModelInfo {
            id: "qwen-plus".to_string(),
            description: "Qwen Plus — 能力均衡，推理效果和速度表现优秀".to_string(),
        },
        ModelInfo {
            id: "qwen-turbo".to_string(),
            description: "Qwen Turbo — 通义千问高速模型，低延迟高并发".to_string(),
        },
        // === 编程专用大模型 (Qwen2.5 Coder) ===
        ModelInfo {
            id: "qwen2.5-coder-32b-instruct".to_string(),
            description: "Qwen2.5 Coder 32B — 顶级的开源代码模型，出色的代码生成与修复能力"
                .to_string(),
        },
        ModelInfo {
            id: "qwen2.5-coder-14b-instruct".to_string(),
            description: "Qwen2.5 Coder 14B — 平衡性能与速度的编程模型".to_string(),
        },
        ModelInfo {
            id: "qwen2.5-coder-7b-instruct".to_string(),
            description: "Qwen2.5 Coder 7B — 轻量级极速代码生成模型".to_string(),
        },
        // === 开源系列大模型 (Qwen2.5) ===
        ModelInfo {
            id: "qwen2.5-72b-instruct".to_string(),
            description: "Qwen2.5 72B — 开源阵营中最强梯队的通用模型".to_string(),
        },
        ModelInfo {
            id: "qwen2.5-14b-instruct".to_string(),
            description: "Qwen2.5 14B — 通用能力均衡开源模型".to_string(),
        },
        ModelInfo {
            id: "qwen2.5-7b-instruct".to_string(),
            description: "Qwen2.5 7B — 轻量级通用开源模型".to_string(),
        },
        // === 多模态视觉模型 ===
        ModelInfo {
            id: "qwen-vl-max".to_string(),
            description: "Qwen VL Max — 视觉理解旗舰模型".to_string(),
        },
        ModelInfo {
            id: "qwen-vl-plus".to_string(),
            description: "Qwen VL Plus — 视觉理解高性能模型".to_string(),
        },
        // === 推理思考模型 (QwQ) ===
        ModelInfo {
            id: "qwen-max-latest".to_string(),
            description: "Qwen Max Latest — 最新版旗舰，具备部分推理与思考加强特性".to_string(),
        },
        ModelInfo {
            id: "qwq-plus".to_string(),
            description: "QwQ Plus — 推理思考模型，128K context".to_string(),
        },
        ModelInfo {
            id: "qwq-32b-preview".to_string(),
            description: "QwQ 32B Preview — 轻量级推理思考模型".to_string(),
        },
    ]
}

#[async_trait]
impl ProviderExtension for QwenExtension {
    fn id(&self) -> &str {
        "qwen"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        // 阿里云 DashScope /models 接口返回数据杂乱，包含许多废弃或无法使用的内部模型
        // 为了更好的体验，我们直接返回精心维护的优质模型列表
        Ok(qwen_models())
    }

    async fn get_balance(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<BalanceStatus>> {
        // 阿里云 DashScope 目前没有公开针对兼容模式的余额查询端点
        Ok(None)
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 阿里云的接口通常提供较高的并发余量，但为了保守起见，依然做合理的限制
        ConcurrencyConfig {
            official_max: 20,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 30, // 遇到并发问题时 AIMD 能探测到的最大上限
            ..Default::default()
        }
    }
}

pub fn extension() -> Arc<QwenExtension> {
    Arc::new(QwenExtension::new())
}
