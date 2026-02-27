pub mod api_key;
pub mod jimeng;
// pub mod cookie;
pub mod anthropic;
pub mod anthropic_oauth;
pub mod antigravity;
pub mod codex_oauth;
pub mod gemini_api_key;
pub mod gemini_cli;
pub mod iflow;
pub mod kimi;
pub mod kling;
pub mod qwen;
pub mod service_account;
pub mod spark;

pub mod ollama;
pub mod xfyun_maas;
pub mod vidu;


pub use anthropic::AnthropicApiKeyAuth;
pub use anthropic_oauth::AnthropicOAuth;
pub use antigravity::{AntigravityOAuth, DynamicOAuthConfig};
pub use api_key::ApiKeyAuth;
pub use codex_oauth::CodexOAuth;
pub use gemini_api_key::GeminiApiKeyAuth;
pub use gemini_cli::GeminiCliOAuth;
pub use iflow::IFlowAuth;
pub use kimi::KimiOAuth;
pub use kling::KlingAuth;
pub use qwen::QwenOAuth;
pub use service_account::ServiceAccountAuth;
pub use spark::SparkAuth;

pub use ollama::OllamaAuth;
pub use xfyun_maas::XfyunMaasAuth;
pub use jimeng::JimengAuth;
pub use vidu::ViduApiKeyAuth;
pub mod baichuan;
pub use baichuan::BaichuanAuth;
