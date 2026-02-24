// 用于兼容文档中的 crate::provider::LlmResponse 引用
use futures::stream::BoxStream;

pub mod extension;
pub mod openai;
pub mod iflow;
pub mod antigravity;
pub mod gemini_cli;
pub mod gemini;
pub mod vertex;
pub mod qwen;
pub mod kimi;
pub mod anthropic;
pub mod codex;
pub mod deepseek;
pub mod moonshot;
pub mod zhipu;
pub mod amp;
pub mod zai;
pub mod openrouter;

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone)]
pub struct LlmChunk {
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub type BoxLlmStream = BoxStream<'static, anyhow::Result<LlmChunk>>;
