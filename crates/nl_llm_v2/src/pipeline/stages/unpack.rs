use async_trait::async_trait;

use crate::pipeline::traits::{Stage, PipelineContext, PipelineInput, PipelineOutput};
use crate::protocol::traits::{ProtocolFormat, ProtocolHook};

use std::sync::Arc;

/// 解包阶段：将服务器返回的原始数据解读为 LlmResponse 或 BoxLlmStream
pub struct UnpackStage {
    protocol: Arc<dyn ProtocolFormat>,
    // [保留] hooks 字段供未来扩展
    // 原因：当前 before_unpack 接受 &mut Value，但本阶段处理的是 Raw bytes/Response
    // 未来可能添加 after_unpack 钩子用于后处理 LlmResponse
    #[allow(dead_code)]
    hooks: Vec<Arc<dyn ProtocolHook>>,
}

impl UnpackStage {
    pub fn new(protocol: Arc<dyn ProtocolFormat>, hooks: Vec<Arc<dyn ProtocolHook>>) -> Self {
        Self { protocol, hooks }
    }
}

#[async_trait]
impl Stage for UnpackStage {
    fn name(&self) -> &str {
        "unpack"
    }

    async fn process(&self, ctx: &mut PipelineContext<'_>) -> anyhow::Result<()> {
        // [修复] 先判断当前 input 类型，避免 match borrow 问题
        // 原因：需要在 match 内部 move ctx.input，所以先检查变体类型
        let is_raw_response = matches!(ctx.input, PipelineInput::RawResponse(_));

        if is_raw_response {
            // [修复] 使用 take_input 取出所有权，避免 move out of mutable reference
            // 原因：RawResponse 包含 reqwest::Response，需要 move ownership
            let input = ctx.take_input();
            if let PipelineInput::RawResponse(resp) = input {
                let stream = self.protocol.unpack_stream(resp)?;
                ctx.output = Some(PipelineOutput::Stream(stream));
            }
        } else {
            match &ctx.input {
                // 非流式：解析字节为 LlmResponse
                PipelineInput::Raw(raw) => {
                    let body_str = String::from_utf8(raw.clone())
                        .unwrap_or_else(|_| String::from("<<Binary Data>>"));

                    let response = self.protocol.unpack_response(&body_str)?;

                    ctx.output = Some(PipelineOutput::Response(response));
                }

                _ => {
                    return Err(anyhow::anyhow!(
                        "UnpackStage expects Raw bytes or RawResponse in PipelineContext, got {:?}",
                        ctx.input.variant_name()
                    ));
                }
            }
        }

        Ok(())
    }
}

// [新增] 辅助方法获取枚举变体名称
// 原因：用于错误信息中显示当前变体类型
impl PipelineInput {
    fn variant_name(&self) -> &'static str {
        match self {
            PipelineInput::Primitive(_) => "Primitive",
            PipelineInput::Packed(_) => "Packed",
            PipelineInput::Raw(_) => "Raw",
            PipelineInput::RawResponse(_) => "RawResponse",
        }
    }
}
