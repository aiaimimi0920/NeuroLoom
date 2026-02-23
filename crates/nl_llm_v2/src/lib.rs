pub mod primitive;
pub mod provider;
pub mod site;
pub mod protocol;
pub mod auth;
pub mod model;
pub mod pipeline;
pub mod presets;
pub mod client;

pub use client::LlmClient;
pub use primitive::request::PrimitiveRequest;
// [新增] 导出常用类型，便于外部使用
pub use site::{Site, SimpleSite, VertexSite, OpenAiSite, CloudCodeSite};
pub use site::context::{UrlContext, Action, AuthType};
pub use protocol::base::{OpenAiProtocol, GeminiProtocol, ClaudeProtocol};
pub use model::{ModelResolver, Capability, DefaultModelResolver};
