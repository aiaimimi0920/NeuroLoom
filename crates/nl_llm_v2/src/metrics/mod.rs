//! 指标收集模块
//!
//! 提供请求响应时间、成功率等运行时指标的收集和存储。

mod pipeline;
mod store;

pub use pipeline::PipelineMetrics;
pub use store::{MetricsStore, MetricsSummary};
