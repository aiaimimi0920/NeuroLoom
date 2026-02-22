use crate::primitive::{PrimitiveRequest, PrimitiveContent, Role};
use serde_json::{json, Value};

pub struct GeminiCompiler;

impl GeminiCompiler {
    pub fn compile(&self, primitive: &PrimitiveRequest) -> Value {
        let mut body = json!({});

        // System instruction
        if let Some(system) = &primitive.system {
            body["systemInstruction"] = json!({
                "parts": [{"text": system}]
            });
        }

        // Contents (messages)
        let contents: Vec<Value> = primitive.messages.iter().map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model", // Gemini uses "model" not "assistant"
                Role::System => "user",
            };

            let parts: Vec<Value> = msg.content.iter().map(|c| {
                match c {
                    PrimitiveContent::Text { text } => json!({"text": text}),
                    PrimitiveContent::Image { mime_type, data } => json!({
                        "inlineData": { "mimeType": mime_type, "data": data }
                    }),
                    PrimitiveContent::ToolCall { name, arguments, .. } => json!({
                        "functionCall": { "name": name, "args": arguments }
                    }),
                    PrimitiveContent::ToolResult { tool_call_id, content, .. } => json!({
                        "functionResponse": { "name": tool_call_id, "response": {"result": content} }
                    }),
                    PrimitiveContent::Thinking { text } => json!({"text": text}),
                }
            }).collect();

            json!({"role": role, "parts": parts})
        }).collect();
        body["contents"] = json!(contents);

        // Generation config
        let mut gen_config = json!({});
        if let Some(max_tokens) = primitive.parameters.max_tokens {
            gen_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(temperature) = primitive.parameters.temperature {
            gen_config["temperature"] = json!(temperature);
        }
        if let Some(top_p) = primitive.parameters.top_p {
            gen_config["topP"] = json!(top_p);
        }
        if let Some(stop) = &primitive.parameters.stop_sequences {
            gen_config["stopSequences"] = json!(stop);
        }
        if gen_config != json!({}) {
            body["generationConfig"] = gen_config;
        }

        // Tools
        if !primitive.tools.is_empty() {
            let func_decls: Vec<Value> = primitive.tools.iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                })
            }).collect();
            body["tools"] = json!([{"functionDeclarations": func_decls}]);
        }

        body
    }
}
