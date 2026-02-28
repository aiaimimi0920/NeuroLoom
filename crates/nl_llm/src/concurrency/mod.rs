//! 并发控制模块
//!
//! 提供基于 AIMD（加性增、乘性减）算法的弹性并发控制，
//! 自动适应平台限流，优化吞吐量。

mod config;
mod controller;
mod permit;

pub use config::{AdjustmentStrategy, ConcurrencyConfig};
pub use controller::{ConcurrencyController, ConcurrencySnapshot, FailureType};
pub use permit::ConcurrencyPermit;
