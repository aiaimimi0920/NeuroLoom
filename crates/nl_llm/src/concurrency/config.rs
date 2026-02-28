//! 并发配置定义

/// 并发配置（静态）
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// 官方声称的最大并发数
    pub official_max: usize,

    /// 初始并发限制（默认为官方值的 50%）
    pub initial_limit: usize,

    /// 最小并发限制（下限）
    pub min_limit: usize,

    /// 最大并发限制（上限，通常等于官方值）
    pub max_limit: usize,

    /// 调节策略
    pub strategy: AdjustmentStrategy,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            official_max: 10,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 10,
            strategy: AdjustmentStrategy::Aimd {
                additive_increment: 1,
                multiplicative_decrease: 0.7,
            },
        }
    }
}

impl ConcurrencyConfig {
    /// 创建新的并发配置
    pub fn new(official_max: usize) -> Self {
        Self {
            official_max,
            initial_limit: official_max / 2,
            min_limit: 1,
            max_limit: official_max,
            strategy: AdjustmentStrategy::Aimd {
                additive_increment: 1,
                multiplicative_decrease: 0.7,
            },
        }
    }

    /// 设置初始并发限制
    pub fn with_initial_limit(mut self, limit: usize) -> Self {
        self.initial_limit = limit;
        self
    }

    /// 设置最小并发限制
    pub fn with_min_limit(mut self, limit: usize) -> Self {
        self.min_limit = limit;
        self
    }

    /// 设置调节策略
    pub fn with_strategy(mut self, strategy: AdjustmentStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

/// 调节策略
#[derive(Debug, Clone)]
pub enum AdjustmentStrategy {
    /// AIMD: 加性增、乘性减（类似 TCP 拥塞控制）
    Aimd {
        /// 每成功 N 次请求后增加的并发量
        additive_increment: usize,
        /// 失败时乘以这个系数（0.0-1.0）
        multiplicative_decrease: f32,
    },

    /// 基于延迟的调节
    LatencyBased {
        /// 目标延迟（毫秒）
        target_latency_ms: u64,
        /// 低于目标延迟 * 此阈值时增加
        increase_threshold: f32,
        /// 高于目标延迟 * 此阈值时减少
        decrease_threshold: f32,
    },

    /// 固定（不调节）
    Fixed,
}
