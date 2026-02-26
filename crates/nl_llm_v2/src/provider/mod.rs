pub mod doubao;
pub mod jimeng;
// 用于兼容文档中的 crate::provider::LlmResponse 引用
use futures::stream::BoxStream;

pub mod balance;
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
pub mod longcat;
pub mod minimax;
pub mod doubaoseed;
pub mod bailing;
pub mod mimo;
pub mod modelscope;
pub mod aihubmix;
pub mod siliconflow;
pub mod dmxapi;
pub mod nvidia;
pub mod packycode;
pub mod cubence;
pub mod aigocode;
pub mod rightcode;
pub mod aicodemirror;
pub mod azure_openai;
pub mod aws_claude;
pub mod cohere;
pub mod cloudflare;
pub mod qianfan;
pub mod perplexity;
pub mod kat_coder;
pub mod spark;
pub mod hunyuan;
pub mod dify;
pub mod jina;
pub mod mistral;
pub mod custom;
pub mod fastgpt;
pub mod aiproxy;
pub mod moka;
pub mod coze;
pub mod kling;

// 显式导出类型，避免 extension() 函数名冲突
pub use balance::*;
pub use extension::*;

pub use amp::AmpExtension;
pub use anthropic::AnthropicExtension;
pub use antigravity::AntigravityExtension;
pub use codex::CodexExtension;
pub use deepseek::DeepSeekExtension;
pub use gemini::GeminiExtension;
pub use gemini_cli::GeminiCliExtension;
pub use iflow::IFlowExtension;
pub use longcat::LongcatExtension;
pub use minimax::MiniMaxExtension;
pub use doubaoseed::DouBaoSeedExtension;
pub use bailing::BaiLingExtension;
pub use mimo::MiMoExtension;
pub use modelscope::ModelScopeExtension;
pub use aihubmix::AiHubMixExtension;
pub use siliconflow::SiliconFlowExtension;
pub use dmxapi::DmxApiExtension;
pub use nvidia::NvidiaExtension;
pub use packycode::PackyCodeExtension;
pub use cubence::CubenceExtension;
pub use aigocode::AiGoCodeExtension;
pub use rightcode::RightCodeExtension;
pub use aicodemirror::AiCodeMirrorExtension;
pub use azure_openai::AzureOpenAiExtension;
pub use aws_claude::AwsClaudeExtension;
pub use cohere::CohereExtension;
pub use cloudflare::CloudflareExtension;
pub use qianfan::QianfanExtension;
pub use perplexity::PerplexityExtension;
pub use kat_coder::KatCoderExtension;
pub use kimi::KimiExtension;
pub use moonshot::MoonshotExtension;
pub use openai::OpenAiExtension;
pub use openrouter::OpenRouterExtension;
pub use qwen::QwenExtension;
pub use vertex::VertexExtension;
pub use zai::ZaiExtension;
pub use zhipu::ZhipuExtension;
pub use fastgpt::FastGptExtension;
pub use coze::CozeExtension;
pub use kling::KlingExtension;

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
