pub mod openai;
pub mod gemini;
pub mod claude;
pub mod codex;

// [新增] Re-export 协议类型，便于外部使用
pub use openai::OpenAiProtocol;
pub use gemini::GeminiProtocol;
pub use claude::ClaudeProtocol;
pub use codex::CodexProtocol;
