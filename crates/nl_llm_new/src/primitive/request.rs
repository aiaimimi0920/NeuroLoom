//! 原语请求定义

use serde::{Deserialize, Serialize};

use super::{PrimitiveMessage, PrimitiveMetadata, PrimitiveParameters, PrimitiveTool};

/// 中间原语格式 - 与任何特定 API 无关的抽象表示
///
/// 设计目的：
/// 1. 解耦输入解析和输出生成
/// 2. 作为格式转换的统一中间层
/// 3. 支持包裹检测和解包
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrimitiveRequest {
    /// 模型标识（不含 provider 前缀）
    pub model: String,

    /// 系统提示词（已解包，纯用户意图）
    ///
    /// 解包后会移除 Claude Code / Gemini CLI 等身份标识
    pub system: Option<String>,

    /// 消息历史（已标准化）
    pub messages: Vec<PrimitiveMessage>,

    /// 工具定义（已标准化，已过滤内置工具）
    pub tools: Vec<PrimitiveTool>,

    /// 生成参数
    #[serde(flatten)]
    pub parameters: PrimitiveParameters,

    /// 元数据（用于追踪和调试）
    #[serde(skip_serializing_if = "PrimitiveMetadata::is_empty")]
    pub metadata: PrimitiveMetadata,
}

impl PrimitiveRequest {
    /// 创建新的原语请求
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// 设置系统提示词
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// 添加消息
    pub fn with_message(mut self, message: PrimitiveMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// 添加工具
    pub fn with_tool(mut self, tool: PrimitiveTool) -> Self {
        self.tools.push(tool);
        self
    }

    /// 设置最大 Token 数
    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.parameters.max_tokens = Some(max_tokens);
        self
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.parameters.temperature = Some(temperature);
        self
    }

    /// 快捷创建：单条用户消息
    pub fn single_user_message(text: impl Into<String>) -> Self {
        Self::default().with_message(PrimitiveMessage::user(text))
    }

    /// 快捷创建：系统提示 + 用户消息
    pub fn with_system_and_user(
        system: impl Into<String>,
        user: impl Into<String>,
    ) -> Self {
        Self::default()
            .with_system(system)
            .with_message(PrimitiveMessage::user(user))
    }
}
