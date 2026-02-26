pub mod claude;
pub mod codex;
pub mod coze;
pub mod dify;
pub mod gemini;
pub mod openai;

// [新增] Re-export 协议类型，便于外部使用
pub use claude::ClaudeProtocol;
pub use codex::CodexProtocol;
pub use coze::CozeProtocol;
pub use dify::DifyProtocol;
pub use gemini::GeminiProtocol;
pub use openai::OpenAiProtocol;
