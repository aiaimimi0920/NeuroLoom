use crate::primitive::{PrimitiveRequest, PrimitiveMetadata};
use crate::translator::{WrapperKind, TranslateError};
use serde_json::Value;

pub fn unwrap(parsed: &Value, wrapper: WrapperKind) -> Result<PrimitiveRequest, TranslateError> {
    let mut request: PrimitiveRequest = Default::default();
    
    request.model = parsed.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    request.metadata = PrimitiveMetadata {
        source_format: crate::translator::Format::OpenAI,
        wrapper_kind: wrapper,
        was_unwrapped: true,
        client_specific: Default::default(),
    };

    Ok(request)
}

pub fn unwrap_response(_parsed: &Value, _wrapper: WrapperKind) -> Result<PrimitiveRequest, TranslateError> {
    // Used when converting an OpenAI response JSON into primitive
    Ok(PrimitiveRequest::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unwrap_openai() {
        let input = json!({ "model": "gpt-4o" });
        let req = unwrap(&input, WrapperKind::None).unwrap();
        assert_eq!(req.model, "gpt-4o");
    }
}
