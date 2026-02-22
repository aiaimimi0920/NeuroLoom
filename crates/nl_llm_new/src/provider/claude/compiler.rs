use crate::primitive::{PrimitiveRequest, PrimitiveContent, Role};
use serde_json::{json, Value};

pub struct ClaudeCompiler;

impl ClaudeCompiler {
    pub fn compile(&self, primitive: &PrimitiveRequest) -> Value {
        let mut body = json!({
            "model": primitive.model,
        });

        // System prompt: Claude uses array of content blocks
        if let Some(system) = &primitive.system {
            body["system"] = json!([{"type": "text", "text": system}]);
        }

        // Messages
        let messages: Vec<Value> = primitive.messages.iter().map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "user", // Claude doesn't have system role in messages
            };

            let content: Vec<Value> = msg.content.iter().map(|c| {
                match c {
                    PrimitiveContent::Text { text } => json!({"type": "text", "text": text}),
                    PrimitiveContent::Image { mime_type, data } => json!({
                        "type": "image",
                        "source": { "type": "base64", "media_type": mime_type, "data": data }
                    }),
                    PrimitiveContent::ToolCall { id, name, arguments } => json!({
                        "type": "tool_use", "id": id, "name": name, "input": arguments
                    }),
                    PrimitiveContent::ToolResult { tool_call_id, content, is_error } => json!({
                        "type": "tool_result", "tool_use_id": tool_call_id, "content": content, "is_error": is_error
                    }),
                    PrimitiveContent::Thinking { text } => json!({"type": "thinking", "thinking": text}),
                }
            }).collect();

            json!({ "role": role, "content": content })
        }).collect();
        body["messages"] = json!(messages);

        // Parameters
        if let Some(max_tokens) = primitive.parameters.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = primitive.parameters.temperature {
            body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = primitive.parameters.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &primitive.parameters.stop_sequences {
            body["stop_sequences"] = json!(stop);
        }

        // Tools
        if !primitive.tools.is_empty() {
            let tools: Vec<Value> = primitive.tools.iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            }).collect();
            body["tools"] = json!(tools);
        }

        body
    }
}
