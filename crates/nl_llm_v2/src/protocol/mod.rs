pub mod traits;
pub mod error;
pub mod base;
pub mod hooks;

pub use base::openai::OpenAiProtocol;
pub use base::gemini::GeminiProtocol;
