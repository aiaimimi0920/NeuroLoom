//! 转换层错误定义

use thiserror::Error;

/// 转换层错误
#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("JSON 解析失败: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("不支持的格式: {0}")]
    UnsupportedFormat(String),

    #[error("缺少必要字段: {0}")]
    MissingField(String),

    #[error("无效的角色类型: {0}")]
    InvalidRole(String),

    #[error("工具转换失败: {0}")]
    ToolConversion(String),

    #[error("内容块转换失败: {0}")]
    ContentConversion(String),

    #[error("不支持的包裹类型: {0}")]
    UnsupportedWrapper(String),
}
