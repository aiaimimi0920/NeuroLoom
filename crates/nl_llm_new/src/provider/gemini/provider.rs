use super::compiler::GeminiCompiler;
use crate::auth::{Auth, ApiKeyConfig, ApiKeyProvider, SAProvider};
use crate::provider::{LlmProvider, LlmResponse, BoxStream, LlmChunk};
use crate::primitive::PrimitiveRequest;
use async_trait::async_trait;

// ─── VertexProvider (Service Account) ───

pub struct VertexProvider {
    compiler: GeminiCompiler,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    credentials_json: String,
    #[allow(dead_code)]
    location: String,
    auth_enum: Auth,
}

impl VertexProvider {
    pub fn from_service_account(credentials_json: String, model: String, location: Option<String>) -> Self {
        let loc = location.unwrap_or_else(|| "us-central1".to_string());
        let auth_enum = Auth::ServiceAccount {
            provider: SAProvider::VertexAI,
            credentials_json: credentials_json.clone(),
        };
        Self {
            compiler: GeminiCompiler,
            model,
            credentials_json,
            location: loc,
            auth_enum,
        }
    }
}

#[async_trait]
impl LlmProvider for VertexProvider {
    fn id(&self) -> &str { "vertex" }
    fn auth(&self) -> &Auth { &self.auth_enum }
    fn supported_models(&self) -> &[&str] {
        &["gemini-1.5-pro", "gemini-1.5-flash", "gemini-2.0-flash", "gemini-2.5-flash", "gemini-2.5-pro"]
    }
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        self.compiler.compile(primitive)
    }
    async fn complete(&self, _body: serde_json::Value) -> crate::Result<LlmResponse> {
        unimplemented!("TDD: implement Vertex AI execution")
    }
    async fn stream(&self, _body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        unimplemented!("TDD: implement stream")
    }
}

// ─── GoogleAIStudioProvider (API Key) ───

pub struct GoogleAIStudioProvider {
    compiler: GeminiCompiler,
    #[allow(dead_code)]
    model: String,
    auth_enum: Auth,
}

impl GoogleAIStudioProvider {
    pub fn from_api_key(api_key: String, model: String) -> Self {
        let auth_enum = Auth::ApiKey(ApiKeyConfig::new(api_key, ApiKeyProvider::GeminiAIStudio));
        Self {
            compiler: GeminiCompiler,
            model,
            auth_enum,
        }
    }
}

#[async_trait]
impl LlmProvider for GoogleAIStudioProvider {
    fn id(&self) -> &str { "google_ai_studio" }
    fn auth(&self) -> &Auth { &self.auth_enum }
    fn supported_models(&self) -> &[&str] {
        &["gemini-1.5-pro", "gemini-1.5-flash", "gemini-2.0-flash", "gemini-2.5-flash", "gemini-2.5-pro"]
    }
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        self.compiler.compile(primitive)
    }
    async fn complete(&self, _body: serde_json::Value) -> crate::Result<LlmResponse> {
        unimplemented!("TDD: implement Google AI Studio execution")
    }
    async fn stream(&self, _body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        unimplemented!("TDD: implement stream")
    }
}
