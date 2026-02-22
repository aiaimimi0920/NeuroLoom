use crate::primitive::PrimitiveRequest;
use crate::translator::TranslateError;
use serde_json::{json, Value};

pub fn wrap(primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
    Ok(json!({
        "model": primitive.model,
        "contents": serde_json::to_value(&primitive.messages)?,
        "systemInstruction": primitive.system
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_gemini_cli() {
        let req = PrimitiveRequest {
            model: "gemini-1.5-pro".to_string(),
            ..Default::default()
        };
        let wrapped = wrap(&req).unwrap();
        assert_eq!(wrapped["model"], "gemini-1.5-pro");
    }
}
