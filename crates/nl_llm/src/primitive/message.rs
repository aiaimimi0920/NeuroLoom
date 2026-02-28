use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };
        write!(f, "{}", s)
    }
}

/// 原语消息
#[derive(Debug, Clone)]
pub struct PrimitiveMessage {
    pub role: Role,
    pub content: Vec<PrimitiveContent>,
}

/// 原语内容块
#[derive(Debug, Clone)]
pub enum PrimitiveContent {
    Text {
        text: String,
    },
    Image {
        url: String,
        mime_type: Option<String>,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}
