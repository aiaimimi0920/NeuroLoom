//! # nl_llm - NeuroLoom LLM Gateway
//!
//! 算力与网关层，实现全局令牌桶反压、基数树前缀缓存、PTY 通信。

pub mod fallback;
pub mod gateway;
pub mod prompt;
pub mod prompt_ast;
pub mod provider;
pub mod token_bucket;

pub use fallback::FallbackRouter;
pub use gateway::{GatewayPreparedRequest, LlmGateway};
pub use prompt::PromptBuilder;
pub use prompt_ast::{PromptAst, PromptNode};
pub use token_bucket::TokenBucket;

pub use nl_core::{NeuroLoomError, Result};
