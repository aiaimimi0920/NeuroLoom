use crate::primitive::{PrimitiveRequest, PrimitiveContent, Role};
use serde_json::{json, Value};

pub struct OpenAICompiler;

impl OpenAICompiler {
    pub fn compile(&self, primitive: &PrimitiveRequest) -> Value {
        let mut body = json!({
            "model": primitive.model,
        });

        let mut messages: Vec<Value> = Vec::new();

        // System message prepended if present
        if let Some(system) = &primitive.system {
            messages.push(json!({"role": "system", "content": system}));
        }

        // Messages
        for msg in &primitive.messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
            };

            // For simple text messages, use string content
            if msg.content.len() == 1 {
                if let PrimitiveContent::Text { text } = &msg.content[0] {
                    messages.push(json!({"role": role, "content": text}));
                    continue;
                }
            }

            // For complex content, use content array
            let content: Vec<Value> = msg.content.iter().map(|c| {
                match c {
                    PrimitiveContent::Text { text } => json!({"type": "text", "text": text}),
                    PrimitiveContent::Image { mime_type, data } => json!({
                        "type": "image_url",
                        "image_url": { "url": format!("data:{};base64,{}", mime_type, data) }
                    }),
                    PrimitiveContent::ToolCall { id, name, arguments } => json!({
                        "type": "function", "id": id, "function": {"name": name, "arguments": arguments.to_string()}
                    }),
                    PrimitiveContent::ToolResult { tool_call_id, content, .. } => json!({
                        "role": "tool", "tool_call_id": tool_call_id, "content": content
                    }),
                    PrimitiveContent::Thinking { text } => json!({"type": "text", "text": text}),
                }
            }).collect();
            messages.push(json!({"role": role, "content": content}));
        }
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
            body["stop"] = json!(stop);
        }

        // Tools
        if !primitive.tools.is_empty() {
            let tools: Vec<Value> = primitive.tools.iter().map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            }).collect();
            body["tools"] = json!(tools);
        }

        body
    }
}
