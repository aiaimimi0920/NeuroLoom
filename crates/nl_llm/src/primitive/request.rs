use serde_json::Value;
use std::collections::HashMap;

use super::{
    message::PrimitiveMessage, metadata::PrimitiveMetadata, parameters::PrimitiveParameters,
    tool::PrimitiveTool,
};

/// 原语请求：统一中间表示
#[derive(Debug, Clone)]
pub struct PrimitiveRequest {
    /// 模型名称（可使用别名，由 ModelResolver 解析）
    pub model: String,

    /// 系统提示
    pub system: Option<String>,

    /// 消息列表
    pub messages: Vec<PrimitiveMessage>,

    /// 工具定义
    pub tools: Vec<PrimitiveTool>,

    /// 生成参数
    pub parameters: PrimitiveParameters,

    /// 元数据
    pub metadata: PrimitiveMetadata,

    /// 是否流式请求
    pub stream: bool,

    /// 平台特定参数
    pub extra: HashMap<String, Value>,
}

impl PrimitiveRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            system: None,
            messages: Vec::new(),
            tools: Vec::new(),
            parameters: PrimitiveParameters::default(),
            metadata: PrimitiveMetadata::default(),
            stream: false,
            extra: HashMap::new(),
        }
    }

    pub fn single_user_message(text: impl Into<String>) -> Self {
        let mut req = Self::new("");
        req.messages.push(PrimitiveMessage {
            role: super::message::Role::User,
            content: vec![super::message::PrimitiveContent::Text { text: text.into() }],
        });
        req
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}
