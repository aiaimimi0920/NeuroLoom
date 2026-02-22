use crate::auth::AuthError;
use std::path::Path;

/// Vertex AI Service Account 认证
pub struct VertexSAAuth {
    #[allow(dead_code)]
    credentials_json: String,
}

impl VertexSAAuth {
    pub fn new(credentials_json: impl Into<String>) -> Self {
        Self {
            credentials_json: credentials_json.into(),
        }
    }

    pub fn from_file(_path: &Path) -> Result<Self, AuthError> {
        Ok(Self::new("{}"))
    }

    pub async fn get_token(&self) -> Result<String, AuthError> {
        // TDD TODO: implement JWT generation
        Ok("TODO_token".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_sa() {
        let auth = VertexSAAuth::new("{}");
        assert_eq!(auth.credentials_json, "{}");
    }
}
