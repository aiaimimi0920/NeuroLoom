//! LLM Gateway 执行编排层（最小实现）
//!
//! 负责：
//! - 接收 Prompt AST
//! - 走令牌桶限流
//! - 按 fallback 当前 provider 编译请求
//! - 提供统一的“请求准备结果”

use tokio::sync::Mutex;

use crate::fallback::FallbackRouter;
use crate::prompt_ast::PromptAst;
use crate::provider::{AnthropicProvider, OllamaProvider, OpenAIProvider};
use crate::token_bucket::TokenBucket;

#[derive(Debug, Clone)]
pub struct GatewayPreparedRequest {
    pub provider: String,
    pub endpoint: String,
    pub body: serde_json::Value,
}

pub struct LlmGateway {
    token_bucket: TokenBucket,
    fallback: Mutex<FallbackRouter>,
}

impl LlmGateway {
    pub fn new(token_bucket: TokenBucket, fallback: FallbackRouter) -> Self {
        Self {
            token_bucket,
            fallback: Mutex::new(fallback),
        }
    }

    pub fn default_gateway() -> Self {
        Self::new(
            TokenBucket::default_bucket(),
            FallbackRouter::default_config(),
        )
    }

    pub async fn prepare_request(&self, ast: &PromptAst) -> crate::Result<GatewayPreparedRequest> {
        self.token_bucket.acquire().await?;

        let router = self.fallback.lock().await;
        let provider = router.current().ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider(
                "no provider available in fallback router".to_string(),
            )
        })?;

        let body = match provider.name.as_str() {
            "openai" => OpenAIProvider::default_provider().compile_request(ast),
            "anthropic" => AnthropicProvider::default_provider().compile_request(ast),
            "ollama" => OllamaProvider::default_provider().compile_request(ast),
            other => {
                self.token_bucket.release();
                return Err(crate::NeuroLoomError::LlmProvider(format!(
                    "unsupported provider in fallback router: {other}"
                )));
            }
        };

        self.token_bucket.release();

        Ok(GatewayPreparedRequest {
            provider: provider.name.clone(),
            endpoint: provider.endpoint.clone(),
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_ast::PromptNode;

    #[tokio::test]
    async fn test_prepare_request_from_default_router() {
        let gateway = LlmGateway::default_gateway();
        let ast = PromptAst::new().push(PromptNode::User("hello".to_string()));

        let req = gateway
            .prepare_request(&ast)
            .await
            .expect("prepare should work");
        assert_eq!(req.provider, "anthropic");
        assert!(req.body.is_object());
    }
}
