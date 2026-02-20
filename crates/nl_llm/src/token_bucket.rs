//! 全局令牌桶流量控制器
//!
//! 防止 LLM API 并发雪崩，实现反压排队机制。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// 令牌桶配置
#[derive(Debug, Clone)]
pub struct TokenBucketConfig {
    /// 桶容量 (最大令牌数)
    pub capacity: u64,
    /// 令牌补充速率 (每秒)
    pub refill_rate: u64,
    /// 单次请求消耗令牌数
    pub tokens_per_request: u64,
    /// 最大等待时间
    pub max_wait: Duration,
}

impl Default for TokenBucketConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            refill_rate: 10,
            tokens_per_request: 1,
            max_wait: Duration::from_secs(60),
        }
    }
}

/// 全局令牌桶
pub struct TokenBucket {
    /// 当前令牌数
    tokens: AtomicU64,
    /// 配置
    config: TokenBucketConfig,
    /// 最后补充时间
    last_refill: Mutex<Instant>,
    /// 并发控制信号量
    semaphore: Semaphore,
}

impl TokenBucket {
    /// 创建新的令牌桶
    pub fn new(config: TokenBucketConfig) -> Self {
        let tokens = AtomicU64::new(config.capacity);
        let semaphore = Semaphore::new(config.capacity as usize);
        Self {
            tokens,
            config,
            last_refill: Mutex::new(Instant::now()),
            semaphore,
        }
    }

    /// 创建默认配置的令牌桶
    pub fn default_bucket() -> Self {
        Self::new(TokenBucketConfig::default())
    }

    /// 获取令牌 (异步等待)
    pub async fn acquire(&self) -> crate::Result<()> {
        // 尝试获取信号量许可
        let permit = tokio::time::timeout(
            self.config.max_wait,
            self.semaphore.acquire(),
        )
        .map_err(|_| crate::NeuroLoomError::TokenBucketExhausted(
            "Timeout waiting for token".to_string()
        ))?
        .map_err(|_| crate::NeuroLoomError::TokenBucketExhausted(
            "Semaphore closed".to_string()
        ))?;

        permit.forget(); // 不自动释放，由 release 控制
        Ok(())
    }

    /// 尝试获取令牌 (非阻塞)
    pub fn try_acquire(&self) -> crate::Result<()> {
        match self.semaphore.try_acquire() {
            Ok(permit) => {
                permit.forget();
                Ok(())
            }
            Err(_) => Err(crate::NeuroLoomError::TokenBucketExhausted(
                "No tokens available".to_string(),
            )),
        }
    }

    /// 释放令牌
    pub fn release(&self) {
        self.semaphore.add_permits(1);
    }

    /// 获取当前令牌数
    pub fn available_tokens(&self) -> u64 {
        self.semaphore.available_permits() as u64
    }

    /// 补充令牌
    pub async fn refill(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.lock().await;
        let elapsed = now.duration_since(*last_refill);

        if elapsed >= Duration::from_secs(1) {
            let tokens_to_add = (elapsed.as_secs() as u64) * self.config.refill_rate;
            let current = self.tokens.load(Ordering::Relaxed);
            let new_count = (current + tokens_to_add).min(self.config.capacity);
            self.tokens.store(new_count, Ordering::Relaxed);

            // 更新信号量
            let permits_to_add = (new_count - current) as usize;
            if permits_to_add > 0 {
                self.semaphore.add_permits(permits_to_add);
            }

            *last_refill = now;
        }
    }

    /// 获取配置
    pub fn config(&self) -> &TokenBucketConfig {
        &self.config
    }
}

/// 请求优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// 带优先级的请求包装
#[derive(Debug, Clone)]
pub struct PrioritizedRequest<T> {
    pub priority: Priority,
    pub request: T,
    pub timestamp: Instant,
}

impl<T> PrioritizedRequest<T> {
    pub fn new(priority: Priority, request: T) -> Self {
        Self {
            priority,
            request,
            timestamp: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let bucket = TokenBucket::default_bucket();

        // 应该成功获取令牌
        assert!(bucket.try_acquire().is_ok());
        bucket.release();
    }

    #[tokio::test]
    async fn test_token_bucket_exhaustion() {
        let config = TokenBucketConfig {
            capacity: 2,
            refill_rate: 0,
            tokens_per_request: 1,
            max_wait: Duration::from_millis(100),
        };
        let bucket = TokenBucket::new(config);

        // 消耗所有令牌
        assert!(bucket.try_acquire().is_ok());
        assert!(bucket.try_acquire().is_ok());

        // 应该失败
        assert!(bucket.try_acquire().is_err());
    }
}
