//! LLM Provider 子模块

pub mod anthropic;
pub mod black_magic_proxy;
pub mod cli_proxy;
pub mod ollama;
pub mod openai;

pub use black_magic_proxy::{
    BlackMagicProxyCatalog, BlackMagicProxyClient, BlackMagicProxySpec, BlackMagicProxyTarget,
    ProxyChatRequest, ProxyExposure, ProxyExposureKind, ProxyMessage, ProxyPreparedCall,
    ProxyPreparedCliCall, ProxyPreparedHttpCall, ProxyPreparedWsCall,
};
pub use cli_proxy::CliProxy;
