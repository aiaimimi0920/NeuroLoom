use crate::primitive::PrimitiveRequest;
use crate::translator::TranslateError;
use serde_json::{json, Value};

pub fn wrap(primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
    Ok(json!({
        "model": primitive.model,
        "instructions": primitive.system,
        "history": serde_json::to_value(&primitive.messages)?
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_codex() {
        let req = PrimitiveRequest {
            model: "gpt-5-codex".to_string(),
            ..Default::default()
        };
        let wrapped = wrap(&req).unwrap();
        assert_eq!(wrapped["model"], "gpt-5-codex");
    }
}
