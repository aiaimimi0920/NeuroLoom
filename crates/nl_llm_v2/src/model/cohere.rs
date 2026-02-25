//! Cohere 模型解析器
//!
//! 提供 Cohere Command 系列模型的别名解析和能力检测。
//!
//! ## 支持的模型
//!
//! | 模型 ID | 说明 | 上下文长度 | 能力 |
//! |---------|------|-----------|------|
//! | `command-a-03-2025` | 最新旗舰模型 | 256K | Chat, Tools, Streaming |
//! | `command-a-vision-07-2025` | 支持图像输入 | 256K | Chat, Tools, Streaming, Vision |
//! | `command-a-reasoning-08-2025` | 推理增强 | 256K | Chat, Tools, Streaming, Thinking |
//! | `command-a-translate-08-2025` | 翻译专用 | 256K | Chat, Tools, Streaming |
//! | `command-r-plus-08-2024` | 强力模型 | 128K | Chat, Tools, Streaming |
//! | `command-r-08-2024` | 平衡模型 | 128K | Chat, Tools, Streaming |
//! | `command-r7b-12-2024` | 轻量快速 | 128K | Chat, Tools, Streaming |
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `cohere` / `command` / `command-a` | command-a-03-2025 |
//! | `vision` | command-a-vision-07-2025 |
//! | `reasoning` | command-a-reasoning-08-2025 |
//! | `translate` | command-a-translate-08-2025 |
//! | `r+` | command-r-plus-08-2024 |
//! | `r` | command-r-08-2024 |
//! | `r7b` | command-r7b-12-2024 |

use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Cohere 模型解析器
pub struct CohereModelResolver {
    inner: DefaultModelResolver,
}

impl CohereModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("cohere", "command-a-03-2025"),
            ("command", "command-a-03-2025"),
            ("command-a", "command-a-03-2025"),
            ("vision", "command-a-vision-07-2025"),
            ("reasoning", "command-a-reasoning-08-2025"),
            ("translate", "command-a-translate-08-2025"),
            ("r+", "command-r-plus-08-2024"),
            ("r", "command-r-08-2024"),
            ("r7b", "command-r7b-12-2024"),
        ]);

        let chat_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let vision_caps = chat_caps | Capability::VISION;
        let reasoning_caps = chat_caps | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("command-a-03-2025", chat_caps),
            ("command-a-vision-07-2025", vision_caps),
            ("command-a-reasoning-08-2025", reasoning_caps),
            ("command-a-translate-08-2025", chat_caps),
            ("command-r-plus-08-2024", chat_caps),
            ("command-r-08-2024", chat_caps),
            ("command-r7b-12-2024", chat_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("command-a-03-2025", 256_000),
            ("command-a-vision-07-2025", 256_000),
            ("command-a-reasoning-08-2025", 256_000),
            ("command-a-translate-08-2025", 256_000),
            ("command-r-plus-08-2024", 128_000),
            ("command-r-08-2024", 128_000),
            ("command-r7b-12-2024", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for CohereModelResolver {
    fn default() -> Self { Self::new() }
}

impl ModelResolver for CohereModelResolver {
    fn resolve(&self, model: &str) -> String { self.inner.resolve(model) }
    fn has_capability(&self, model: &str, cap: Capability) -> bool { self.inner.has_capability(model, cap) }
    fn max_context(&self, model: &str) -> usize { self.inner.max_context(model) }
    fn context_window_hint(&self, model: &str) -> (usize, usize) { self.inner.context_window_hint(model) }
}
