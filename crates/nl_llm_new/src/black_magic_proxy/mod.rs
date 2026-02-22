//! 黑魔法代理接口聚合层
//!
//! 统一四类代理输出形态：
//! - API（HTTP JSON）
//! - Auth（鉴权代理头/票据）
//! - WebSocket（实时双工）
//! - CLI（本地命令行代理）
//!
//! 对应上游项目：CLIProxyAPI / newapi / ccswitch / Claude Code Router。

mod types;
mod catalog;
mod client;

pub use types::*;
pub use catalog::*;
pub use client::*;
