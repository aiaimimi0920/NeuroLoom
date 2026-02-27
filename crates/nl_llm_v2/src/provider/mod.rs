pub mod doubao;
pub mod jimeng;
pub mod replicate;
pub mod sora;
pub mod vidu;
// 用于兼容文档中的 crate::provider::LlmResponse 引用
use futures::stream::BoxStream;

pub mod aicodemirror;
pub mod aigocode;
pub mod aihubmix;
pub mod aiproxy;
pub mod amp;
pub mod anthropic;
pub mod antigravity;
pub mod aws_claude;
pub mod azure_openai;
pub mod bailing;
pub mod balance;
pub mod cloudflare;
pub mod codex;
pub mod cohere;
pub mod coze;
pub mod cubence;
pub mod custom;
pub mod deepseek;
pub mod dify;
pub mod dmxapi;
pub mod doubaoseed;
pub mod extension;
pub mod fastgpt;
pub mod gemini;
pub mod gemini_cli;
pub mod hunyuan;
pub mod iflow;
pub mod jina;
pub mod kat_coder;
pub mod kimi;
pub mod kling;
pub mod longcat;
pub mod mimo;
pub mod minimax;
pub mod mistral;
pub mod modelscope;
pub mod moka;
pub mod moonshot;
pub mod nvidia;
pub mod openai;
pub mod openrouter;
pub mod packycode;
pub mod perplexity;
pub mod qianfan;
pub mod qwen;
pub mod rightcode;
pub mod siliconflow;
pub mod spark;
pub mod vertex;

pub mod ollama;
pub mod xfyun_maas;
pub mod zai;
pub mod zhipu;

// 显式导出类型，避免 extension() 函数名冲突
pub use balance::*;
pub use extension::*;

pub use aicodemirror::AiCodeMirrorExtension;
pub use aigocode::AiGoCodeExtension;
pub use aihubmix::AiHubMixExtension;
pub use amp::AmpExtension;
pub use anthropic::AnthropicExtension;
pub use antigravity::AntigravityExtension;
pub use aws_claude::AwsClaudeExtension;
pub use azure_openai::AzureOpenAiExtension;
pub use bailing::BaiLingExtension;
pub use cloudflare::CloudflareExtension;
pub use codex::CodexExtension;
pub use cohere::CohereExtension;
pub use coze::CozeExtension;
pub use cubence::CubenceExtension;
pub use deepseek::DeepSeekExtension;
pub use dmxapi::DmxApiExtension;
pub use doubaoseed::DouBaoSeedExtension;
pub use fastgpt::FastGptExtension;
pub use gemini::GeminiExtension;
pub use gemini_cli::GeminiCliExtension;
pub use iflow::IFlowExtension;
pub use kat_coder::KatCoderExtension;
pub use kimi::KimiExtension;
pub use kling::KlingExtension;
pub use longcat::LongcatExtension;
pub use mimo::MiMoExtension;
pub use minimax::MiniMaxExtension;
pub use modelscope::ModelScopeExtension;
pub use moonshot::MoonshotExtension;
pub use nvidia::NvidiaExtension;
pub use openai::OpenAiExtension;
pub use openrouter::OpenRouterExtension;
pub use packycode::PackyCodeExtension;
pub use perplexity::PerplexityExtension;
pub use qianfan::QianfanExtension;
pub use qwen::QwenExtension;
pub use rightcode::RightCodeExtension;
pub use siliconflow::SiliconFlowExtension;
pub use vertex::VertexExtension;
pub use vidu::ViduExtension;

pub use ollama::OllamaExtension;
pub use xfyun_maas::XfyunMaasExtension;
pub use zai::ZaiExtension;
pub use zhipu::ZhipuExtension;

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
