//! Cloudflare Workers AI 模型解析器
//!
//! 提供云端边缘网络运行的 AI 模型别名解析和能力检测。
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文长度 | 能力 |
//! |---------|------|-----------|------|
//! | `@cf/meta/llama-3.3-70b-instruct-fp8-fast` | Llama 3.3 70B FP8 — 最新最强 | 131K | Chat, Tools, Streaming |
//! | `@cf/meta/llama-3.1-70b-instruct` | Llama 3.1 70B — 强力模型 | 131K | Chat, Tools, Streaming |
//! | `@cf/meta/llama-3.1-8b-instruct` | Llama 3.1 8B — 免费快速 | 131K | Chat, Tools, Streaming |
//! | `@cf/deepseek-ai/deepseek-r1-distill-qwen-32b` | DeepSeek R1 — 推理模型 | 65K | Chat, Thinking, Streaming |
//! | `@cf/mistral/mistral-7b-instruct-v0.2-lora` | Mistral 7B — 轻量对话 | 32K | Chat, Streaming |
//! | `@cf/qwen/qwen1.5-14b-chat-awq` | Qwen 1.5 14B — 中文优化 | 32K | Chat, Streaming |
//! | `@hf/google/gemma-7b-it` | Gemma 7B — Google 开源 | 8K | Chat, Streaming |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `cloudflare` / `llama` / `llama-3.3` | @cf/meta/llama-3.3-70b-instruct-fp8-fast |
//! | `llama-70b` | @cf/meta/llama-3.1-70b-instruct |
//! | `llama-8b` | @cf/meta/llama-3.1-8b-instruct |
//! | `deepseek` / `r1` | @cf/deepseek-ai/deepseek-r1-distill-qwen-32b |
//! | `mistral` | @cf/mistral/mistral-7b-instruct-v0.2-lora |
//! | `qwen` | @cf/qwen/qwen1.5-14b-chat-awq |
//! | `gemma` | @hf/google/gemma-7b-it |

use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Cloudflare Workers AI 模型解析器
pub struct CloudflareModelResolver {
    inner: DefaultModelResolver,
}

impl CloudflareModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("cloudflare", "@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
            ("llama", "@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
            ("llama-3.3", "@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
            ("llama-70b", "@cf/meta/llama-3.1-70b-instruct"),
            ("llama-8b", "@cf/meta/llama-3.1-8b-instruct"),
            ("deepseek", "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b"),
            ("r1", "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b"),
            ("mistral", "@cf/mistral/mistral-7b-instruct-v0.2-lora"),
            ("qwen", "@cf/qwen/qwen1.5-14b-chat-awq"),
            ("gemma", "@hf/google/gemma-7b-it"),
        ]);

        let chat_caps = Capability::CHAT | Capability::STREAMING;
        let tool_caps = chat_caps | Capability::TOOLS;
        let reasoning = chat_caps | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("@cf/meta/llama-3.3-70b-instruct-fp8-fast", tool_caps),
            ("@cf/meta/llama-3.1-70b-instruct", tool_caps),
            ("@cf/meta/llama-3.1-8b-instruct", tool_caps),
            ("@cf/deepseek-ai/deepseek-r1-distill-qwen-32b", reasoning),
            ("@cf/mistral/mistral-7b-instruct-v0.2-lora", chat_caps),
            ("@cf/qwen/qwen1.5-14b-chat-awq", chat_caps),
            ("@hf/google/gemma-7b-it", chat_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("@cf/meta/llama-3.3-70b-instruct-fp8-fast", 131_072),
            ("@cf/meta/llama-3.1-70b-instruct", 131_072),
            ("@cf/meta/llama-3.1-8b-instruct", 131_072),
            ("@cf/deepseek-ai/deepseek-r1-distill-qwen-32b", 65_536),
            ("@cf/mistral/mistral-7b-instruct-v0.2-lora", 32_768),
            ("@cf/qwen/qwen1.5-14b-chat-awq", 32_768),
            ("@hf/google/gemma-7b-it", 8_192),
        ]);

        Self { inner }
    }
}

impl Default for CloudflareModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for CloudflareModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.inner.resolve(model)
    }
    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        self.inner.has_capability(model, cap)
    }
    fn max_context(&self, model: &str) -> usize {
        self.inner.max_context(model)
    }
    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        self.inner.context_window_hint(model)
    }
    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        None
    }
}
