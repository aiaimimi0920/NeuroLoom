use async_trait::async_trait;
use serde_json::Value;

use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxLlmStream, LlmResponse};
use crate::site::context::UrlContext;

#[async_trait]
pub trait Stage: Send + Sync {
    /// 阶段名称
    fn name(&self) -> &str;

    /// 处理数据
    async fn process(&self, context: &mut PipelineContext<'_>) -> anyhow::Result<()>;
}

pub enum PipelineInput {
    /// 原语请求
    Primitive(PrimitiveRequest),

    /// 封包数据
    Packed(Value),

    /// 原始字节（非流式响应）
    Raw(Vec<u8>),

    /// [新增] 原始 HTTP 响应（流式）
    /// 原因：流式请求需要保存 Response 对象供 unpack_stream 使用
    RawResponse(reqwest::Response),
}

pub enum PipelineOutput {
    /// 响应
    Response(LlmResponse),

    /// 流式响应
    Stream(BoxLlmStream),
}

/// 流水线上下文
pub struct PipelineContext<'a> {
    /// 输入数据（可能是原语或封包数据）
    pub input: PipelineInput,

    /// 输出数据
    pub output: Option<PipelineOutput>,

    /// 当前阶段
    pub current_stage: usize,

    /// 是否直通
    pub passthrough: bool,

    /// URL 构建上下文
    pub url_context: UrlContext<'a>,

    /// 由 AuthenticateStage 透传的授权相关的附加元数据 (如 project_id)
    pub auth_extra: std::collections::HashMap<String, serde_json::Value>,
}

impl<'a> PipelineContext<'a> {
    pub fn from_primitive(req: PrimitiveRequest, url_context: UrlContext<'a>) -> Self {
        Self {
            input: PipelineInput::Primitive(req),
            output: None,
            current_stage: 0,
            passthrough: false,
            url_context,
            auth_extra: std::collections::HashMap::new(),
        }
    }

    /// [新增] 取出 input 并替换为占位符
    /// 原因：用于需要 move ownership 的场景（如 RawResponse）
    pub fn take_input(&mut self) -> PipelineInput {
        // [修复] 使用 Raw(vec![]) 作为占位符，因为它是安全的默认值
        std::mem::replace(&mut self.input, PipelineInput::Raw(vec![]))
    }

    pub fn take_response(&mut self) -> anyhow::Result<LlmResponse> {
        match self.output.take() {
            Some(PipelineOutput::Response(resp)) => Ok(resp),
            _ => Err(anyhow::anyhow!(
                "Expected Response output, got something else or none"
            )),
        }
    }

    pub fn take_stream(&mut self) -> anyhow::Result<BoxLlmStream> {
        match self.output.take() {
            Some(PipelineOutput::Stream(stream)) => Ok(stream),
            _ => Err(anyhow::anyhow!(
                "Expected Stream output, got something else or none"
            )),
        }
    }
}
