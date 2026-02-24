use crate::pipeline::traits::{PipelineContext, PipelineInput};
use crate::protocol::traits::ProtocolHook;
use serde_json::Value;

/// iFlow Thinking 钩子
///
/// iFlow 原生支持部分模型的推理拆分，通过以下策略注入：
/// - GLM 系列：`chat_template_kwargs.enable_thinking: true` + `clear_thinking: false`
/// - Qwen/DeepSeek 系列：`chat_template_kwargs.enable_thinking: true`
/// - MiniMax 系列：`reasoning_split: true`
pub struct IflowThinkingHook;

impl IflowThinkingHook {
    /// 判断模型是否支持 enable_thinking 模式
    fn is_thinking_model(model: &str) -> bool {
        // GLM 系列全部支持
        if model.starts_with("glm") {
            return true;
        }
        // 特定模型支持
        matches!(model,
            "qwen3-max" | "qwen3-max-preview"
            | "deepseek-v3" | "deepseek-v3.1" | "deepseek-v3.2"
            | "deepseek-r1"
        )
    }

    /// 判断是否为 GLM 模型（需要额外的 clear_thinking 参数）
    fn is_glm_model(model: &str) -> bool {
        model.starts_with("glm")
    }

    /// 判断是否为 MiniMax 模型（使用 reasoning_split 而非 enable_thinking）
    fn is_minimax_model(model: &str) -> bool {
        model.starts_with("minimax")
    }
}

impl ProtocolHook for IflowThinkingHook {
    fn after_pack(&self, ctx: &mut PipelineContext<'_>, packed: &mut Value) {
        if let PipelineInput::Primitive(primitive) = &ctx.input {
            let model = primitive.model.to_lowercase();

            if Self::is_thinking_model(&model) {
                if let Some(obj) = packed.as_object_mut() {
                    let kwargs = obj.entry("chat_template_kwargs").or_insert(serde_json::json!({}));
                    if let Some(kwargs_obj) = kwargs.as_object_mut() {
                        kwargs_obj.insert("enable_thinking".to_string(), Value::Bool(true));
                        // GLM 模型需要 clear_thinking: false 以保留推理过程
                        if Self::is_glm_model(&model) {
                            kwargs_obj.insert("clear_thinking".to_string(), Value::Bool(false));
                        }
                    }
                }
            } else if Self::is_minimax_model(&model) {
                if let Some(obj) = packed.as_object_mut() {
                    obj.insert("reasoning_split".to_string(), Value::Bool(true));
                }
            }
        }
    }
}
