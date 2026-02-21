//! LLM Provider 子模块

pub mod anthropic;
pub mod antigravity;
pub mod black_magic_proxy;
pub mod cli_proxy;
pub mod gemini_cli;
pub mod gemini_common;
pub mod google_ai_studio;
pub mod iflow;
pub mod ollama;
pub mod openai;
pub mod vertex;
pub mod vertex_compat;

pub use black_magic_proxy::{
    BlackMagicProxyCatalog, BlackMagicProxyClient, BlackMagicProxySpec, BlackMagicProxyTarget,
    ProxyChatRequest, ProxyExposure, ProxyExposureKind, ProxyMessage, ProxyPreparedCall,
    ProxyPreparedCliCall, ProxyPreparedHttpCall, ProxyPreparedWsCall,
};
pub use cli_proxy::CliProxy;
pub use gemini_cli::{GeminiCliConfig, GeminiCliProvider};
pub use google_ai_studio::{GoogleAIStudioConfig, GoogleAIStudioProvider};
pub use iflow::{
    extract_bx_auth, should_refresh_api_key, IFlowConfig, IFlowProvider, IFlowRefreshResult,
    IFlowTokenStorage,
};
pub use vertex::{VertexConfig, VertexProvider};
pub use vertex_compat::{VertexCompatConfig, VertexCompatProvider};
