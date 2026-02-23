use async_trait::async_trait;

use crate::pipeline::traits::{Stage, PipelineContext, PipelineInput};
use crate::protocol::traits::{ProtocolFormat, ProtocolHook};

use std::sync::Arc;

/// 封包阶段：将 PrimitiveRequest 转换为目标格式
pub struct PackStage {
    protocol: Arc<dyn ProtocolFormat>,
    hooks: Vec<Arc<dyn ProtocolHook>>,
}

impl PackStage {
    pub fn new(protocol: Arc<dyn ProtocolFormat>, hooks: Vec<Arc<dyn ProtocolHook>>) -> Self {
        Self { protocol, hooks }
    }
}

#[async_trait]
impl Stage for PackStage {
    fn name(&self) -> &str {
        "pack"
    }

    async fn process(&self, ctx: &mut PipelineContext<'_>) -> anyhow::Result<()> {
        if let PipelineInput::Primitive(primitive) = &ctx.input {
            let is_stream = primitive.stream;

            // 封包
            let mut packed = self.protocol.pack(primitive, is_stream);

            // 合并 extra 参数
            for (key, value) in &primitive.extra {
                packed[key] = value.clone();
            }

            // 应用协议变体钩子
            for hook in &self.hooks {
                hook.after_pack(ctx, &mut packed);
            }

            ctx.input = PipelineInput::Packed(packed);
        }

        Ok(())
    }
}
