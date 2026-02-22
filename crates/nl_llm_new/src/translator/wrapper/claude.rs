use crate::primitive::PrimitiveRequest;
use crate::translator::TranslateError;
use serde_json::{json, Value};

pub fn wrap(primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
    Ok(json!({
        "model": primitive.model,
        "system": primitive.system.clone().unwrap_or_default(),
        "messages": serde_json::to_value(&primitive.messages)?
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_claude() {
        let req = PrimitiveRequest {
            model: "claude-3".to_string(),
            ..Default::default()
        };
        let wrapped = wrap(&req).unwrap();
        assert_eq!(wrapped["model"], "claude-3");
    }
}
