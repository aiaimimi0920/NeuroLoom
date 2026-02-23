use serde_json::Value;

/// 原语工具定义
#[derive(Debug, Clone)]
pub struct PrimitiveTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}
