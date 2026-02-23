use crate::pipeline::traits::{PipelineContext, PipelineInput};
use crate::protocol::traits::ProtocolHook;
use serde_json::Value;

/// iFlow 原生支持模型的推理拆分
pub struct IflowThinkingHook;

impl ProtocolHook for IflowThinkingHook {
    fn after_pack(&self, ctx: &mut PipelineContext<'_>, packed: &mut Value) {
        if let PipelineInput::Primitive(primitive) = &ctx.input {
            let model = primitive.model.to_lowercase();
            // Thinking models
            let is_thinking = ["glm-4-plus", "glm-4-air", "glm-4-airx", "glm-4-flash", "glm-4-long", "qwen3-max", "deepseek-v3", "deepseek-r1"]
                .iter()
                .any(|m| model.starts_with(m)) || model.starts_with("glm");

            // Split reasoning models
            let needs_reasoning_split = ["minimax-01"].iter().any(|m| model.starts_with(m));

            if is_thinking {
                if let Some(obj) = packed.as_object_mut() {
                    let kwargs = obj.entry("chat_template_kwargs").or_insert(serde_json::json!({}));
                    if let Some(kwargs_obj) = kwargs.as_object_mut() {
                        kwargs_obj.insert("enable_thinking".to_string(), Value::Bool(true));
                        if model.starts_with("glm") {
                            kwargs_obj.insert("clear_thinking".to_string(), Value::Bool(false));
                        }
                    }
                }
            } else if needs_reasoning_split {
                if let Some(obj) = packed.as_object_mut() {
                    obj.insert("reasoning_split".to_string(), Value::Bool(true));
                }
            }
        }
    }
}
