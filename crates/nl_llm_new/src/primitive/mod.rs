//! 原语格式（中间表示）
//!
//! 采用编译器式中间表示（IR）架构，避免 N×(N-1) 组合爆炸。
//! 与任何特定 API 无关的抽象表示。

mod request;
mod message;
mod tool;
mod parameters;
mod metadata;

pub use request::*;
pub use message::*;
pub use tool::*;
pub use parameters::*;
pub use metadata::*;
