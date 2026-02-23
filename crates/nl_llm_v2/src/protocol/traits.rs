use serde_json::Value;

use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, BoxLlmStream};
use crate::protocol::error::StandardError;
use crate::pipeline::traits::PipelineContext;

/// 协议格式定义
pub trait ProtocolFormat: Send + Sync {
    /// 协议标识
    fn id(&self) -> &str;

    /// 封包：PrimitiveRequest → JSON
    /// is_stream 参数用于在 JSON body 中添加 stream 标识
    fn pack(&self, primitive: &PrimitiveRequest, is_stream: bool) -> Value;

    /// 解包响应：JSON → LlmResponse
    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse>;

    /// 解包流式响应
    fn unpack_stream(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<BoxLlmStream>;

    /// 检测格式是否匹配（用于直通优化）
    fn matches_format(&self, data: &Value) -> bool;

    /// 解包错误：将平台错误转换为标准错误
    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError>;
}

/// 协议钩子（扩展签名，可访问 PipelineContext）
pub trait ProtocolHook: Send + Sync {
    /// 封包后处理
    fn after_pack(&self, ctx: &mut PipelineContext, packed: &mut Value) {
        let _ = (ctx, packed);
    }

    /// 解包前处理
    fn before_unpack(&self, ctx: &mut PipelineContext, data: &mut Value) {
        let _ = (ctx, data);
    }

    /// 发送前处理（可修改 headers）
    fn before_send(&self, ctx: &mut PipelineContext, req: &mut reqwest::RequestBuilder) {
        let _ = (ctx, req);
    }

    /// 接收后处理（流式响应预处理）
    fn after_receive(&self, ctx: &mut PipelineContext, resp: &mut reqwest::Response) {
        let _ = (ctx, resp);
    }
}
