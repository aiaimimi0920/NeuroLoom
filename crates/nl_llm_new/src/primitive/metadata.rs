//! 元数据定义

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::translator::{Format, WrapperKind};

/// 原语元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrimitiveMetadata {
    /// 原始格式
    #[serde(default)]
    pub source_format: Format,

    /// 检测到的包裹类型
    #[serde(default)]
    pub wrapper_kind: WrapperKind,

    /// 是否被解包
    #[serde(default)]
    pub was_unwrapped: bool,

    /// 原始请求中的客户端特有字段（保留用于回填）
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub client_specific: HashMap<String, serde_json::Value>,
}

impl PrimitiveMetadata {
    /// 创建新的元数据
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置源格式
    pub fn with_source_format(mut self, format: Format) -> Self {
        self.source_format = format;
        self
    }

    /// 设置包裹类型
    pub fn with_wrapper_kind(mut self, kind: WrapperKind) -> Self {
        self.wrapper_kind = kind;
        self
    }

    /// 设置是否被解包
    pub fn with_unwrapped(mut self, unwrapped: bool) -> Self {
        self.was_unwrapped = unwrapped;
        self
    }

    /// 添加客户端特有字段
    pub fn with_client_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.client_specific.insert(key.into(), value);
        self
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        !self.was_unwrapped
            && self.wrapper_kind == WrapperKind::None
            && self.client_specific.is_empty()
    }
}
