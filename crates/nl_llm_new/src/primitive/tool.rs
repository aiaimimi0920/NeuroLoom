//! 工具定义

use serde::{Deserialize, Serialize};

/// 标准化工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveTool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 输入参数 Schema (JSON Schema)
    pub input_schema: serde_json::Value,
}

impl PrimitiveTool {
    /// 创建新的工具定义
    pub fn new(name: impl Into<String>, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 从 JSON 创建
    pub fn from_json(json: &serde_json::Value) -> Option<Self> {
        let name = json.get("name")?.as_str()?.to_string();
        let description = json.get("description").and_then(|d| d.as_str()).map(String::from);
        let input_schema = json.get("input_schema")?.clone();

        Some(Self {
            name,
            description,
            input_schema,
        })
    }
}
