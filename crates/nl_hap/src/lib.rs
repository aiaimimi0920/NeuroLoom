//! # nl_hap - NeuroLoom Hyper-Agent Protocol
//!
//! 星际联邦协议：HAP 跨网竞标、WebSocket 通信、Agent 互操作。

pub mod protocol;
pub mod server;
pub mod client;
pub mod market;

pub use protocol::{HapMessage, HapProtocol};
pub use server::HapServer;
pub use client::HapClient;
pub use market::AgentMarket;
