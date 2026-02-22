use crate::provider::{LlmProvider, LlmResponse};
use crate::primitive::PrimitiveRequest;
use async_trait::async_trait;
use futures::stream::BoxStream;
use crate::provider::LlmChunk;

pub struct CodexProvider;

#[async_trait]
impl LlmProvider for CodexProvider {
    fn id(&self) -> &str {
        "codex"
    }

    fn auth(&self) -> &crate::auth::Auth {
         unimplemented!("TDD: implement root auth")
    }

    fn supported_models(&self) -> &[&str] {
        &["gpt-5-codex"]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        crate::translator::wrapper::codex::wrap(primitive).unwrap_or_default()
    }

    async fn complete(&self, _body: serde_json::Value) -> crate::Result<LlmResponse> {
        unimplemented!("TDD: implement real execution")
    }

    async fn stream(&self, _body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
         unimplemented!("TDD: implement stream")
    }
}
