use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::client::ClientBuilder;

type BuilderFn = fn() -> ClientBuilder;

pub struct PresetRegistry {
    builders: HashMap<&'static str, BuilderFn>,
}

impl PresetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            builders: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.builders.insert("openai", super::openai::builder);
        self.builders.insert("claude", super::claude::builder);
        self.builders.insert("gemini", super::gemini::builder);
        self.builders.insert("vertex", super::vertex::builder);
        self.builders.insert("deepseek", super::deepseek::builder);
        self.builders.insert("moonshot", super::moonshot::builder);
        self.builders.insert("zhipu", super::zhipu::builder);
        self.builders.insert("iflow", super::iflow::builder);
        self.builders
            .insert("openrouter", super::openrouter::builder);
        self.builders
            .insert("gemini_cli", super::gemini_cli::builder);
        self.builders
            .insert("vertex_api", super::vertex_api::builder);
        self.builders.insert("qwen", super::qwen::builder);
        self.builders
            .insert("qwen_coder", super::qwen_coder::builder);
        self.builders
            .insert("qwen_oauth", super::qwen_oauth::builder);
        self.builders.insert("kimi", super::kimi::builder);
        self.builders
            .insert("kimi_coding", super::kimi_coding::builder);
        self.builders
            .insert("kimi_oauth", super::kimi_oauth::builder);
        self.builders
            .insert("claude_oauth", super::claude_oauth::builder);
        self.builders
            .insert("codex_oauth", super::codex_oauth::builder);
        self.builders.insert("codex_api", super::codex_api::builder);
        self.builders
            .insert("antigravity", super::antigravity::builder);
        self.builders.insert("amp", super::amp::builder);
        self.builders.insert("zai", super::zai::builder);
        self.builders.insert("longcat", super::longcat::builder);
        self.builders.insert("minimax", super::minimax::builder);
        self.builders
            .insert("minimax_cn", super::minimax_cn::builder);
        self.builders
            .insert("doubaoseed", super::doubaoseed::builder);
        self.builders.insert("bailing", super::bailing::builder);
        self.builders.insert("mimo", super::mimo::builder);
        self.builders
            .insert("modelscope", super::modelscope::builder);
        self.builders.insert("aihubmix", super::aihubmix::builder);
        self.builders
            .insert("siliconflow", super::siliconflow::builder);
        self.builders
            .insert("siliconflow_en", super::siliconflow_en::builder);
        self.builders.insert("dmxapi", super::dmxapi::builder);
        self.builders.insert("nvidia", super::nvidia::builder);
        self.builders.insert("packycode", super::packycode::builder);
        self.builders.insert("cubence", super::cubence::builder);
        self.builders.insert("aigocode", super::aigocode::builder);
        self.builders.insert("rightcode", super::rightcode::builder);
        self.builders
            .insert("aicodemirror", super::aicodemirror::builder);
        self.builders
            .insert("azure_openai", super::azure_openai::builder);
        self.builders
            .insert("aws_claude", super::aws_claude::builder);
        self.builders
            .insert("aws_claude_ak", super::aws_claude::builder_ak);
        self.builders.insert("cohere", super::cohere::builder);
        self.builders
            .insert("tencent_ti", super::tencent_ti::builder);
        self.builders.insert("hunyuan", super::hunyuan::builder);
        self.builders
            .insert("cloudflare", super::cloudflare::builder);
        self.builders.insert("qianfan", super::qianfan::builder);
        self.builders
            .insert("perplexity", super::perplexity::builder);
        self.builders.insert("kat_coder", super::kat_coder::builder);
        self.builders.insert("spark", super::spark::builder);
        self.builders.insert("spark_x", super::spark::builder_x);
        self.builders.insert("dify", super::dify::builder);
        self.builders.insert("jina", super::jina::builder);
        self.builders.insert("mistral", super::mistral::builder);
        self.builders.insert("custom", super::custom::builder);
        self.builders
            .insert("fastgpt", super::fastgpt::default_builder);
        self.builders.insert("aiproxy", super::aiproxy::builder);
        self.builders.insert("moka", super::moka::builder);
        self.builders.insert("coze", super::coze::default_builder);
        self.builders.insert("kling", super::kling::builder);
        self.builders
            .insert("jimeng", super::jimeng::default_preset);
        self.builders.insert("submodel", super::submodel::builder);
        self.builders.insert("doubao", super::doubao::builder);
        self.builders.insert("sora", super::sora::builder);
        self.builders.insert("vidu", super::vidu::builder);
        self.builders.insert("replicate", super::replicate::builder);
        self.builders.insert("groq", super::groq::builder);
        self.builders.insert("together", super::together::builder);
        self.builders.insert("fireworks", super::fireworks::builder);
        self.builders.insert("ollama", super::ollama::builder);
        self.builders.insert("baichuan", super::baichuan::builder);
        self.builders.insert("cerebras", super::cerebras::builder);
        self.builders.insert("lmstudio", super::lmstudio::builder);
        self.builders.insert("infini", super::infini::builder);
        self.builders.insert("wuwenxq", super::infini::builder);
        self.builders.insert("wuwen_xq", super::infini::builder);
        self.builders
            .insert("huggingface", super::huggingface::builder);
        self.builders.insert("stepfun", super::stepfun::builder);
        self.builders
            .insert("github_models", super::github_models::builder);
        self.builders
            .insert("hyperbolic", super::hyperbolic::builder);
        self.builders.insert("ppio", super::ppio::builder);
        self.builders
            .insert("vercel_ai_gateway", super::vercel_ai_gateway::builder);
        self.builders.insert("302.ai", super::a302::builder);
        self.builders.insert("a302", super::a302::builder);
    }

    pub fn get_builder(&self, preset_name: &str) -> Option<ClientBuilder> {
        self.builders.get(preset_name).map(|f| f())
    }

    pub fn list(&self) -> Vec<&str> {
        self.builders.keys().copied().collect()
    }
}

pub static REGISTRY: Lazy<PresetRegistry> = Lazy::new(PresetRegistry::new);
