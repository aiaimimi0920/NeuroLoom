use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

#[derive(Clone, Copy)]
pub struct CephalonModelSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub capabilities: Capability,
    pub context_window: usize,
}

pub fn cephalon_model_specs() -> Vec<CephalonModelSpec> {
    vec![
        CephalonModelSpec {
            id: "gpt-4o",
            description: "GPT-4o — Flagship multimodal model",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 128_000,
        },
        CephalonModelSpec {
            id: "gpt-4o-mini",
            description: "GPT-4o Mini — Fast and affordable",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 128_000,
        },
        CephalonModelSpec {
            id: "gpt-4-turbo",
            description: "GPT-4 Turbo — Previous generation, 128K context",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 128_000,
        },
        CephalonModelSpec {
            id: "gpt-3.5-turbo",
            description: "GPT-3.5 Turbo — Fast and economical",
            capabilities: Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            context_window: 16_385,
        },
        CephalonModelSpec {
            id: "claude-3-opus-20240229",
            description: "Claude 3 Opus — Most capable Claude model",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 200_000,
        },
        CephalonModelSpec {
            id: "claude-3-sonnet-20240229",
            description: "Claude 3 Sonnet — Balanced performance",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 200_000,
        },
        CephalonModelSpec {
            id: "claude-3-haiku-20240307",
            description: "Claude 3 Haiku — Fast and efficient",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 200_000,
        },
        CephalonModelSpec {
            id: "deepseek-chat",
            description: "DeepSeek Chat — 通用对话模型",
            capabilities: Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            context_window: 64_000,
        },
        CephalonModelSpec {
            id: "deepseek-reasoner",
            description: "DeepSeek Reasoner — 深度推理模型",
            capabilities: Capability::CHAT | Capability::STREAMING | Capability::THINKING,
            context_window: 64_000,
        },
        CephalonModelSpec {
            id: "gemini-1.5-pro",
            description: "Gemini 1.5 Pro — Google's multimodal model",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 1_000_000,
        },
        CephalonModelSpec {
            id: "gemini-1.5-flash",
            description: "Gemini 1.5 Flash — Fast multimodal model",
            capabilities: Capability::CHAT
                | Capability::VISION
                | Capability::TOOLS
                | Capability::STREAMING,
            context_window: 1_000_000,
        },
    ]
}

/// Cephalon 模型解析器
///
/// Cephalon 是一个 AI 模型聚合平台，支持多种主流 LLM 模型。
///
/// 支持的模型类别：
/// - OpenAI 系列：gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-3.5-turbo
/// - Claude 系列：claude-3-opus, claude-3-sonnet, claude-3-haiku
/// - DeepSeek 系列：deepseek-chat, deepseek-reasoner
/// - Gemini 系列：gemini-1.5-pro, gemini-1.5-flash
pub struct CephalonModelResolver {
    inner: DefaultModelResolver,
}

impl CephalonModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // OpenAI 系列别名
            ("gpt4", "gpt-4o"),
            ("gpt-4", "gpt-4o"),
            ("gpt4o", "gpt-4o"),
            ("gpt4-mini", "gpt-4o-mini"),
            ("gpt-4-turbo", "gpt-4-turbo"),
            ("gpt3", "gpt-3.5-turbo"),
            ("gpt-3.5", "gpt-3.5-turbo"),
            ("gpt35", "gpt-3.5-turbo"),
            // Claude 系列别名
            ("claude3-opus", "claude-3-opus-20240229"),
            ("claude-3-opus", "claude-3-opus-20240229"),
            ("claude3-sonnet", "claude-3-sonnet-20240229"),
            ("claude-3-sonnet", "claude-3-sonnet-20240229"),
            ("claude3-haiku", "claude-3-haiku-20240307"),
            ("claude-3-haiku", "claude-3-haiku-20240307"),
            // DeepSeek 系列别名
            ("deepseek", "deepseek-chat"),
            ("ds", "deepseek-chat"),
            ("reasoner", "deepseek-reasoner"),
            ("r1", "deepseek-reasoner"),
            // Gemini 系列别名
            ("gemini", "gemini-1.5-pro"),
            ("gemini-pro", "gemini-1.5-pro"),
            ("gemini-flash", "gemini-1.5-flash"),
        ]);

        let specs = cephalon_model_specs();
        inner.extend_capabilities(
            specs
                .iter()
                .map(|spec| (spec.id, spec.capabilities))
                .collect(),
        );
        inner.extend_context_lengths(
            specs
                .iter()
                .map(|spec| (spec.id, spec.context_window))
                .collect(),
        );

        Self { inner }
    }
}

impl Default for CephalonModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for CephalonModelResolver {
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
