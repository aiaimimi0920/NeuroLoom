use async_trait::async_trait;
use crate::pipeline::traits::{Stage, PipelineContext, PipelineInput};

/// 原语化阶段：将输入转换为 PrimitiveRequest
///
/// # 设计说明
///
/// 在当前架构中，用户直接传入 `PrimitiveRequest`，因此此阶段主要作为占位符。
/// 未来可扩展支持：
///
/// 1. **格式检测**: 自动检测输入数据的协议格式（OpenAI/Claude/Gemini）
/// 2. **自动解包**: 将已封包的数据解包为 `PrimitiveRequest`
/// 3. **格式转换**: 在不同协议间进行转换
///
/// # 当前行为
///
/// - `Primitive`: 直接通过（无需转换）
/// - `Packed`: 返回错误（未实现自动解包）
/// - `Raw`/`RawResponse`: 返回错误（不应出现在此阶段）
///
/// # 扩展方向
///
/// 如需实现自动解包，需要注入：
/// - `FormatDetector`: 检测输入数据的协议格式
/// - `Unwrapper`: 根据格式将数据解包为原语
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
                // [未实现] 自动检测格式并解包
                // 当前架构下，用户应直接传入 PrimitiveRequest
                // 如需支持 Packed 输入，需要：
                // 1. FormatDetector 检测 JSON 结构
                // 2. 根据 detected format 选择对应的 Unwrapper
                // 3. Unwrapper 将 JSON 转换为 PrimitiveRequest
                Err(anyhow::anyhow!(
                    "Unpacking packed data to primitives is not implemented. \
                    Please pass PrimitiveRequest directly instead of pre-packed JSON."
                ))
            }
            PipelineInput::Raw(_) => {
                // Raw bytes 不应该出现在 primitivize 阶段
                // 原因：这是 SendStage 的输出，不应回到原语化
                Err(anyhow::anyhow!("Raw bytes should not appear in primitivize stage"))
            }
            PipelineInput::RawResponse(_) => {
                // RawResponse 不应该出现在 primitivize 阶段
                // 原因：这是 SendStage 的流式输出，不应回到原语化
                Err(anyhow::anyhow!("RawResponse should not appear in primitivize stage"))
            }
        }
    }
}
