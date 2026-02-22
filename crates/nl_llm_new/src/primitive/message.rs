//! 消息定义

use serde::{Deserialize, Serialize};

/// 标准化消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveMessage {
    /// 角色
    pub role: Role,
    /// 内容块列表
    pub content: Vec<PrimitiveContent>,
}

impl PrimitiveMessage {
    /// 创建用户消息
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![PrimitiveContent::Text { text: text.into() }],
        }
    }

    /// 创建助手消息
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![PrimitiveContent::Text { text: text.into() }],
        }
    }

    /// 创建系统消息
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![PrimitiveContent::Text { text: text.into() }],
        }
    }

    /// 添加内容块
    pub fn with_content(mut self, content: PrimitiveContent) -> Self {
        self.content.push(content);
        self
    }
}

/// 角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}

/// 标准化内容块
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PrimitiveContent {
    /// 文本内容
    #[serde(rename = "text")]
    Text { text: String },

    /// 图片内容
    #[serde(rename = "image")]
    Image {
        mime_type: String,
        data: String, // Base64
    },

    /// 工具调用
    #[serde(rename = "tool_use")]
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },

    /// 工具调用结果
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_call_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    /// 思考内容（用于推理模型）
    #[serde(rename = "thinking")]
    Thinking { text: String },
}

impl PrimitiveContent {
    /// 创建文本内容
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// 创建图片内容
    pub fn image(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            mime_type: mime_type.into(),
            data: data.into(),
        }
    }

    /// 创建工具调用
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self::ToolCall {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }

    /// 创建工具调用结果
    pub fn tool_result(
        tool_call_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self::ToolResult {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            is_error,
        }
    }
}
