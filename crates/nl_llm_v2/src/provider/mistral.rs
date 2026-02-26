use serde_json::Value;

use crate::model::resolver::{ModelResolver, Capability};
use crate::pipeline::traits::PipelineContext;
use crate::protocol::traits::ProtocolHook;

/// Mistral 模型解析器
#[derive(Debug, Clone, Default)]
pub struct MistralModelResolver {}

impl MistralModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for MistralModelResolver {
    fn resolve(&self, model: &str) -> String {
        model.to_string()
    }

    fn has_capability(&self, _model: &str, cap: Capability) -> bool {
        // 大多数 Mistral 核心模型均支持文本生成与流式输出
        cap.contains(Capability::CHAT) || cap.contains(Capability::STREAMING)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 大部分常用 Mistral 模型如 mistral-large-latest 是 32k 甚至 128k
        32_768
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        let input_limit = max * 3 / 4;
        let output_limit = max - input_limit;
        (input_limit, output_limit)
    }
}

/// Mistral 的专属请求拦截器（Hook）
/// Mistral API 强制要求 `tool_call_id` 必须严格是 9 位字母或数字： "^[a-zA-Z0-9]{9}$"
pub struct MistralHook {}

impl MistralHook {
    pub fn new() -> Self {
        Self {}
    }

    // 内部帮助函数：生成 9 位随机字母数字字符串
    fn generate_9_char_id() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..9)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    // 内部帮助函数：检查字符串是否仅包含 9 位字母数字
    fn is_mistral_valid_id(s: &str) -> bool {
        if s.len() != 9 {
            return false;
        }
        s.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

impl ProtocolHook for MistralHook {
    fn after_pack(&self, _ctx: &mut PipelineContext, packed: &mut Value) {
        // payload 应为一个 JSON 对象，并在内部包含 "messages" 数组
        if let Some(messages) = packed.get_mut("messages").and_then(|m| m.as_array_mut()) {
            let mut id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            for msg in messages.iter_mut() {
                // 如果消息带有 tool_calls 数组 (Assistant 产生)
                if let Some(tool_calls) = msg.get_mut("tool_calls").and_then(|t| t.as_array_mut()) {
                    for tool_call in tool_calls.iter_mut() {
                        if let Some(id_val) = tool_call.get("id").and_then(|id| id.as_str()) {
                            let original_id = id_val.to_string();
                            if !Self::is_mistral_valid_id(&original_id) {
                                // 查找或生成一个新的
                                let new_id = id_map.entry(original_id).or_insert_with(|| Self::generate_9_char_id());
                                tool_call["id"] = Value::String(new_id.clone());
                            }
                        }
                    }
                }

                // 如果消息有 tool_call_id (Tool 角色发回的结果)
                if let Some(tool_call_id_val) = msg.get("tool_call_id").and_then(|id| id.as_str()) {
                    let original_id = tool_call_id_val.to_string();
                    if let Some(new_id) = id_map.get(&original_id) {
                        // 如果我们在前面重新映射过这个 id，我们就替换它
                        msg["tool_call_id"] = Value::String(new_id.clone());
                    } else if !Self::is_mistral_valid_id(&original_id) {
                        // 有可能因为顺序问题？如果 tool_call_id 没找到且自身不合法，强行生成
                        let new_id = Self::generate_9_char_id();
                        id_map.insert(original_id, new_id.clone());
                        msg["tool_call_id"] = Value::String(new_id);
                    }
                }
            }
        }
    }
}
