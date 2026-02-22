use crate::primitive::{PrimitiveRequest, PrimitiveMetadata};
use crate::translator::{WrapperKind, TranslateError};
use serde_json::Value;

pub fn unwrap(parsed: &Value, wrapper: WrapperKind) -> Result<PrimitiveRequest, TranslateError> {
    let mut request: PrimitiveRequest = Default::default();
    
    request.model = parsed.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    request.metadata = PrimitiveMetadata {
        source_format: crate::translator::Format::Gemini,
        wrapper_kind: wrapper,
        was_unwrapped: true,
        client_specific: Default::default(),
    };

    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unwrap_gemini() {
        let input = json!({ "model": "gemini-1.5" });
        let req = unwrap(&input, WrapperKind::None).unwrap();
        assert_eq!(req.model, "gemini-1.5");
    }
}
