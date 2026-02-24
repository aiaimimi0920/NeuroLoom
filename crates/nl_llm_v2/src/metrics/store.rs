//! 指标存储

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::collections::VecDeque;

use super::pipeline::PipelineMetrics;

/// 指标存储（滑动窗口）
pub struct MetricsStore {
    /// 最近 N 次请求的指标
    recent_metrics: RwLock<VecDeque<PipelineMetrics>>,

    /// 窗口大小
    window_size: usize,

    /// 累计总请求数
    total_requests: AtomicU64,

    /// 累计错误数
    total_errors: AtomicU64,

    /// 累计延迟（毫秒）
    total_latency_ms: AtomicU64,
}

impl MetricsStore {
    /// 创建新的指标存储
    pub fn new(window_size: usize) -> Self {
        Self {
            recent_metrics: RwLock::new(VecDeque::with_capacity(window_size)),
            window_size,
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
        }
    }

    /// 记录指标
    pub fn record(&self, metrics: PipelineMetrics) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if !metrics.success {
            self.total_errors.fetch_add(1, Ordering::Relaxed);
        }

        if let Some(latency) = metrics.response_time_ms {
            self.total_latency_ms.fetch_add(latency, Ordering::Relaxed);
        }

        let mut recent = self.recent_metrics.write().unwrap();
        if recent.len() >= self.window_size {
            recent.pop_front();
        }
        recent.push_back(metrics);
    }

    /// 获取平均响应时间（毫秒）
    pub fn avg_latency_ms(&self) -> u64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0;
        }
        self.total_latency_ms.load(Ordering::Relaxed) / total
    }

    /// 获取成功率（0.0 - 1.0）
    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        let errors = self.total_errors.load(Ordering::Relaxed);
        (total - errors) as f64 / total as f64
    }

    /// 获取统计摘要
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            avg_latency_ms: self.avg_latency_ms(),
            success_rate: self.success_rate(),
            recent_count: self.recent_metrics.read().unwrap().len(),
        }
    }

    /// 获取最近 N 次请求的平均延迟
    pub fn recent_avg_latency_ms(&self, n: usize) -> Option<u64> {
        let recent = self.recent_metrics.read().unwrap();
        let count = n.min(recent.len());
        if count == 0 {
            return None;
        }

        let sum: u64 = recent.iter().rev().take(count)
            .filter_map(|m| m.response_time_ms)
            .sum();

        Some(sum / count as u64)
    }

    /// 获取最近 N 次请求的成功率
    pub fn recent_success_rate(&self, n: usize) -> Option<f64> {
        let recent = self.recent_metrics.read().unwrap();
        let count = n.min(recent.len());
        if count == 0 {
            return None;
        }

        let successes: usize = recent.iter().rev().take(count)
            .filter(|m| m.success)
            .count();

        Some(successes as f64 / count as f64)
    }

    /// 重置统计
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.total_errors.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
        self.recent_metrics.write().unwrap().clear();
    }
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new(100)
    }
}

/// 指标摘要
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    /// 总请求数
    pub total_requests: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: u64,
    /// 成功率（0.0 - 1.0）
    pub success_rate: f64,
    /// 最近请求样本数
    pub recent_count: usize,
}

impl MetricsSummary {
    /// 格式化为可读字符串
    pub fn format(&self) -> String {
        format!(
            "总请求: {}, 错误: {}, 成功率: {:.1}%, 平均延迟: {}ms",
            self.total_requests,
            self.total_errors,
            self.success_rate * 100.0,
            self.avg_latency_ms
        )
    }
}
