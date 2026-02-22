use crate::primitive::PrimitiveRequest;
use crate::translator::TranslateError;
use serde_json::{json, Value};

pub fn wrap(primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
    Ok(json!({
        "model": primitive.model,
        "messages": serde_json::to_value(&primitive.messages)?
    }))
}

pub fn wrap_response(_primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
    Ok(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_openai() {
        let req = PrimitiveRequest {
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        let wrapped = wrap(&req).unwrap();
        assert_eq!(wrapped["model"], "gpt-4");
    }
}
