use crate::primitive::PrimitiveRequest;
use crate::translator::TranslateError;
use serde_json::{json, Value};

pub fn wrap(primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
    Ok(json!({
        "model": primitive.model,
        "systemInstruction": primitive.system,
        "conversation": serde_json::to_value(&primitive.messages)?
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_antigravity() {
        let req = PrimitiveRequest {
            model: "antigravity-1".to_string(),
            ..Default::default()
        };
        let wrapped = wrap(&req).unwrap();
        assert_eq!(wrapped["model"], "antigravity-1");
    }
}
