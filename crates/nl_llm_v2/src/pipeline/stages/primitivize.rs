use async_trait::async_trait;
use crate::pipeline::traits::{Stage, PipelineContext, PipelineInput};

/// 原语化阶段：将输入转换为 PrimitiveRequest
// 占位符结构：后续可以在这里注入 FormatDetector 和 unwrappers
pub struct PrimitivizeStage {}

#[async_trait]
impl Stage for PrimitivizeStage {
    fn name(&self) -> &str {
        "primitivize"
    }

    async fn process(&self, _ctx: &mut PipelineContext<'_>) -> anyhow::Result<()> {
        match &_ctx.input {
            PipelineInput::Primitive(_) => {
                // 已经是原语，跳过
                Ok(())
            }
            PipelineInput::Packed(_data) => {
                // TODO: 检测格式并解包
                Err(anyhow::anyhow!("Unpacking raw bytes to primitives is not implemented yet in scaffolding"))
            }
            PipelineInput::Raw(_) => {
                // [修复] Raw bytes 不应该出现在 primitivize 阶段
                // 原因：这是 SendStage 的输出，不应回到原语化
                Err(anyhow::anyhow!("Raw bytes should not appear in primitivize stage"))
            }
            // [新增] RawResponse 分支处理
            // 原因：PipelineInput 枚举需要处理所有变体
            PipelineInput::RawResponse(_) => {
                Err(anyhow::anyhow!("RawResponse should not appear in primitivize stage"))
            }
        }
    }
}
