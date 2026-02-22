//! 令牌桶限流器

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// 令牌桶限流器
pub struct TokenBucket {
    /// 桶容量
    capacity: u64,
    /// 当前令牌数
    tokens: AtomicU64,
    /// 令牌补充速率（每纳秒）
    refill_rate_nanos: f64,
    /// 上次补充时间
    last_refill: std::sync::Mutex<Instant>,
}

impl TokenBucket {
    /// 创建新的令牌桶
    ///
    /// # 参数
    /// - `capacity`: 桶容量（最大令牌数）
    /// - `refill_interval`: 补充间隔
    pub fn new(capacity: u32, refill_interval: Duration) -> Self {
        Self {
            capacity: capacity as u64,
            tokens: AtomicU64::new(capacity as u64),
            refill_rate_nanos: capacity as f64 / refill_interval.as_nanos() as f64,
            last_refill: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// 尝试获取一个令牌
    ///
    /// 如果成功返回 true，否则返回 false
    pub fn try_acquire(&self) -> bool {
        self.refill();

        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
            if self.tokens.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ).is_ok() {
                return true;
            }
        }
    }

    /// 获取一个令牌（异步等待）
    pub async fn acquire(&self) {
        while !self.try_acquire() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// 尝试获取多个令牌
    pub fn try_acquire_n(&self, n: u64) -> bool {
        self.refill();

        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current < n {
                return false;
            }
            if self.tokens.compare_exchange(
                current,
                current - n,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ).is_ok() {
                return true;
            }
        }
    }

    /// 补充令牌
    fn refill(&self) {
        let mut last_refill = self.last_refill.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        if elapsed.as_nanos() > 0 {
            let new_tokens = (elapsed.as_nanos() as f64 * self.refill_rate_nanos) as u64;
            if new_tokens > 0 {
                let current = self.tokens.load(Ordering::Relaxed);
                let updated = (current + new_tokens).min(self.capacity);
                self.tokens.store(updated, Ordering::Relaxed);
                *last_refill = now;
            }
        }
    }

    /// 获取当前令牌数
    pub fn available(&self) -> u64 {
        self.refill();
        self.tokens.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_basic() {
        let bucket = TokenBucket::new(10, Duration::from_secs(1));

        // 应该能获取 10 个令牌
        for _ in 0..10 {
            assert!(bucket.try_acquire());
        }

        // 第 11 个应该失败
        assert!(!bucket.try_acquire());
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(5, Duration::from_millis(100));

        // 消耗所有令牌
        for _ in 0..5 {
            assert!(bucket.try_acquire());
        }
        assert!(!bucket.try_acquire());

        // 等待补充
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 应该有新令牌了
        assert!(bucket.try_acquire());
    }
}
