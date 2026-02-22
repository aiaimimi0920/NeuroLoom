use super::{config::OpenAIConfig, compiler::OpenAICompiler};
use crate::auth::Auth;
use crate::provider::{LlmProvider, LlmResponse};
use crate::primitive::PrimitiveRequest;
use async_trait::async_trait;
use crate::provider::BoxStream;
use crate::provider::LlmChunk;

pub struct OpenAIProvider {
    config: OpenAIConfig,
    compiler: OpenAICompiler,
    #[allow(dead_code)]
    http: reqwest::Client,
    auth_enum: Auth,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Self {
        let auth_enum = Auth::ApiKey(config.auth.clone());
        Self {
            config,
            compiler: OpenAICompiler,
            http: reqwest::Client::new(),
            auth_enum,
        }
    }

    pub fn is_official(&self) -> bool {
        self.config.auth.base_url.is_none()
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &["gpt-4o", "gpt-4o-mini", "o1-preview", "o1-mini", "gpt-4-turbo"]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        self.compiler.compile(primitive)
    }

    async fn complete(&self, _body: serde_json::Value) -> crate::Result<LlmResponse> {
        unimplemented!("TDD: implement real HTTP execution")
    }

    async fn stream(&self, _body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        unimplemented!("TDD: implement stream")
    }
}
