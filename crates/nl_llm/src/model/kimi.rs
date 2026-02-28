use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// Kimi/Moonshot 平台专属模型解析器
///
/// ## 支持的主流模型
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `moonshot-v1-32k` | Chat, Tools, Streaming | 32K | 标准通用 |
/// | `kimi-k2.5`       | Chat, Tools, Streaming | 128K | 新版旗舰大模型 |
/// | `kimi-for-coding` | Chat, Tools, Streaming | 128K | 代码专用 |
///
/// ## 常用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `kimi` / `k2.5` | `kimi-k2.5` | 最新旗舰大核 |
/// | `coding` / `code` | `kimi-for-coding` | 编码专家 |
pub struct KimiModelResolver {
    inner: DefaultModelResolver,
}

impl KimiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 模型别名 ==========
        inner.extend_aliases(vec![
            // 基础系列
            ("moonshot", "moonshot-v1-32k"),
            ("moonshot-8k", "moonshot-v1-8k"),
            ("moonshot-32k", "moonshot-v1-32k"),
            ("moonshot-128k", "moonshot-v1-128k"),
            // K2.5 最新系列
            ("kimi", "kimi-k2.5"),
            ("k2.5", "kimi-k2.5"),
            ("kimi-k2.5", "kimi-k2.5"),
            // 专属代码大模型 (通常可能跑在 api.kimi.com 节点下)
            ("coding", "kimi-for-coding"),
            ("code", "kimi-for-coding"),
        ]);

        // ========== 能力配置 ==========
        // 绝大部分支持对话、流式和工具调用 (Function Calling)
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![
            ("moonshot-v1-8k", standard_caps),
            ("moonshot-v1-32k", standard_caps),
            ("moonshot-v1-128k", standard_caps),
            ("kimi-k2.5", standard_caps),
            ("kimi-for-coding", standard_caps),
        ]);

        // ========== 上下文长度 ==========
        inner.extend_context_lengths(vec![
            ("moonshot-v1-8k", 8_192),
            ("moonshot-v1-32k", 32_768),
            ("moonshot-v1-128k", 131_072),
            ("kimi-k2.5", 131_072),
            ("kimi-for-coding", 131_072),
        ]);

        Self { inner }
    }
}

impl Default for KimiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for KimiModelResolver {
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
