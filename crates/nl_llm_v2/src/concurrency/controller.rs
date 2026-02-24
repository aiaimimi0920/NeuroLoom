//! 并发控制器

use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::collections::VecDeque;
use tokio::sync::Semaphore;

use super::config::{ConcurrencyConfig, AdjustmentStrategy};

/// 并发控制器运行时状态
pub struct ConcurrencyState {
    /// 当前并发限制
    pub current_limit: AtomicUsize,

    /// 当前活跃请求数
    pub active_requests: AtomicUsize,

    /// 成功请求计数
    pub success_count: AtomicU64,

    /// 失败请求计数
    pub failure_count: AtomicU64,

    /// 最近响应时间样本（滑动窗口）
    pub recent_latencies: RwLock<VecDeque<u64>>,

    /// 是否处于恢复模式
    pub recovering: AtomicBool,

    /// 上次成功后连续成功次数（用于 AIMD）
    pub consecutive_successes: AtomicU64,
}

impl ConcurrencyState {
    pub fn new(initial_limit: usize) -> Self {
        Self {
            current_limit: AtomicUsize::new(initial_limit),
            active_requests: AtomicUsize::new(0),
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            recent_latencies: RwLock::new(VecDeque::with_capacity(100)),
            recovering: AtomicBool::new(false),
            consecutive_successes: AtomicU64::new(0),
        }
    }

    /// 添加延迟样本
    pub fn add_latency_sample(&self, latency_ms: u64) {
        let mut latencies = self.recent_latencies.write().unwrap();
        if latencies.len() >= 100 {
            latencies.pop_front();
        }
        latencies.push_back(latency_ms);
    }

    /// 计算平均延迟
    pub fn average_latency(&self) -> Option<u64> {
        let latencies = self.recent_latencies.read().unwrap();
        if latencies.is_empty() {
            return None;
        }
        let sum: u64 = latencies.iter().sum();
        Some(sum / latencies.len() as u64)
    }
}

/// 并发控制器
pub struct ConcurrencyController {
    config: ConcurrencyConfig,
    state: ConcurrencyState,
    semaphore: Arc<Semaphore>,
}

/// 失败类型
#[derive(Debug, Clone, Copy)]
pub enum FailureType {
    /// 429 限流错误
    RateLimited,
    /// 请求超时
    Timeout,
    /// 服务端错误
    ServerError,
    /// 其他错误
    Other,
}

impl FailureType {
    /// 获取降级系数
    pub fn decrease_factor(&self) -> f32 {
        match self {
            FailureType::RateLimited => 0.5,
            FailureType::Timeout => 0.7,
            FailureType::ServerError => 0.8,
            FailureType::Other => 0.9,
        }
    }
}

/// 并发状态快照
#[derive(Debug, Clone)]
pub struct ConcurrencySnapshot {
    /// 官方最大并发数
    pub official_max: usize,
    /// 当前实际限制
    pub current_limit: usize,
    /// 当前活跃请求数
    pub active_requests: usize,
    /// 累计成功请求数
    pub success_count: u64,
    /// 累计失败请求数
    pub failure_count: u64,
    /// 平均响应延迟（毫秒）
    pub avg_latency_ms: Option<u64>,
}

impl ConcurrencyController {
    /// 创建新的并发控制器
    pub fn new(config: ConcurrencyConfig) -> Self {
        let initial = config.initial_limit;
        Self {
            config,
            state: ConcurrencyState::new(initial),
            semaphore: Arc::new(Semaphore::new(initial)),
        }
    }

    /// 获取许可证（开始请求前调用）
    pub async fn acquire(&self) -> super::permit::ConcurrencyPermit<'_> {
        let permit = self.semaphore.acquire().await.unwrap();
        self.state.active_requests.fetch_add(1, Ordering::Relaxed);
        super::permit::ConcurrencyPermit::new(self, permit)
    }

    /// 尝试获取许可证（非阻塞）
    pub fn try_acquire(&self) -> Option<super::permit::ConcurrencyPermit<'_>> {
        let permit = self.semaphore.try_acquire().ok()?;
        self.state.active_requests.fetch_add(1, Ordering::Relaxed);
        Some(super::permit::ConcurrencyPermit::new(self, permit))
    }

    /// 内部：释放活跃请求计数
    pub(crate) fn release_active(&self) {
        self.state.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    /// 报告请求成功
    pub fn report_success(&self, latency_ms: u64) {
        self.state.success_count.fetch_add(1, Ordering::Relaxed);
        self.state.add_latency_sample(latency_ms);
        self.state.recovering.store(false, Ordering::Relaxed);

        // 根据策略判断是否增加并发
        let should_increase = match &self.config.strategy {
            AdjustmentStrategy::Aimd { additive_increment, .. } => {
                let consecutive = self.state.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
                // 每 N 次成功增加一次并发
                consecutive % (*additive_increment as u64) == 0
            }
            AdjustmentStrategy::LatencyBased { target_latency_ms, increase_threshold, .. } => {
                // 延迟低于目标 * 阈值时增加
                latency_ms < (*target_latency_ms as f32 * increase_threshold) as u64
            }
            AdjustmentStrategy::Fixed => false,
        };

        if should_increase {
            self.increase_limit();
        }
    }

    /// 报告请求失败
    pub fn report_failure(&self, error_type: FailureType) {
        self.state.failure_count.fetch_add(1, Ordering::Relaxed);
        self.state.consecutive_successes.store(0, Ordering::Relaxed);
        self.state.recovering.store(true, Ordering::Relaxed);

        let decrease_factor = match &self.config.strategy {
            AdjustmentStrategy::Aimd { multiplicative_decrease, .. } => {
                // 对 429 错误使用更激进的降级
                let base_factor = error_type.decrease_factor();
                base_factor * multiplicative_decrease
            }
            AdjustmentStrategy::LatencyBased { decrease_threshold, .. } => {
                error_type.decrease_factor() * decrease_threshold
            }
            AdjustmentStrategy::Fixed => return,
        };

        self.decrease_limit(decrease_factor);
    }

    /// 增加并发限制
    fn increase_limit(&self) {
        let current = self.state.current_limit.load(Ordering::Relaxed);
        if current < self.config.max_limit {
            let new_limit = (current + 1).min(self.config.max_limit);
            self.state.current_limit.store(new_limit, Ordering::Relaxed);
            // 增加信号量容量
            self.semaphore.add_permits(1);
        }
    }

    /// 减少并发限制
    fn decrease_limit(&self, factor: f32) {
        let current = self.state.current_limit.load(Ordering::Relaxed);
        let new_limit = ((current as f32 * factor) as usize).max(self.config.min_limit);

        if new_limit < current {
            self.state.current_limit.store(new_limit, Ordering::Relaxed);
            // 注意：Semaphore 不支持直接减少容量
            // 新请求会因为获取不到 permit 而等待
            // 这里我们不主动关闭 permits，只是不再发新的
        }
    }

    /// 获取状态快照
    pub fn snapshot(&self) -> ConcurrencySnapshot {
        ConcurrencySnapshot {
            official_max: self.config.official_max,
            current_limit: self.state.current_limit.load(Ordering::Relaxed),
            active_requests: self.state.active_requests.load(Ordering::Relaxed),
            success_count: self.state.success_count.load(Ordering::Relaxed),
            failure_count: self.state.failure_count.load(Ordering::Relaxed),
            avg_latency_ms: self.state.average_latency(),
        }
    }

    /// 获取当前并发限制
    pub fn current_limit(&self) -> usize {
        self.state.current_limit.load(Ordering::Relaxed)
    }

    /// 获取配置
    pub fn config(&self) -> &ConcurrencyConfig {
        &self.config
    }
}
