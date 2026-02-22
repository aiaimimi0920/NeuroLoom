//! Gateway 编排层
//!
//! 负责：
//! - 全局令牌桶限流
//! - 通用错误重试（429/5xx）
//! - 跨 Provider 降级
//! - 请求超时控制

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmProvider, LlmResponse, ProviderError};
use crate::translator::Format;
use crate::token_bucket::TokenBucket;
use crate::fallback::{FallbackRouter, FallbackConfig};

/// Gateway 配置
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// 全局 QPS 限制
    pub global_qps: u32,
    /// 单 Provider QPS 限制
    pub per_provider_qps: u32,
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试基础延迟（毫秒）
    pub retry_base_delay_ms: u64,
    /// 是否启用降级
    pub enable_fallback: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            global_qps: 100,
            per_provider_qps: 30,
            timeout_secs: 120,
            max_retries: 3,
            retry_base_delay_ms: 500,
            enable_fallback: true,
        }
    }
}

/// Gateway 主结构
pub struct Gateway {
    config: GatewayConfig,
    providers: Arc<RwLock<HashMap<String, Arc<dyn LlmProvider>>>>,
    provider_order: Arc<RwLock<Vec<String>>>,
    global_bucket: TokenBucket,
    provider_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    #[allow(dead_code)]
    fallback_router: FallbackRouter,
}

impl Gateway {
    /// 创建新的 Gateway
    pub fn new(config: GatewayConfig) -> Self {
        let global_bucket = TokenBucket::new(config.global_qps, Duration::from_secs(1));
        let fallback_router = FallbackRouter::new(FallbackConfig::default());

        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
            provider_order: Arc::new(RwLock::new(Vec::new())),
            global_bucket,
            provider_buckets: Arc::new(RwLock::new(HashMap::new())),
            fallback_router,
        }
    }

    /// 注册 Provider
    pub async fn register_provider(&self, provider: Arc<dyn LlmProvider>) {
        let id = provider.id().to_string();
        let bucket = TokenBucket::new(self.config.per_provider_qps, Duration::from_secs(1));

        {
            let mut providers = self.providers.write().await;
            providers.insert(id.clone(), provider);
        }

        {
            let mut buckets = self.provider_buckets.write().await;
            buckets.insert(id.clone(), bucket);
        }

        {
            let mut order = self.provider_order.write().await;
            if !order.contains(&id) {
                order.push(id);
            }
        }
    }

    /// 设置 Provider 优先级顺序
    pub async fn set_provider_order(&self, order: Vec<String>) {
        let mut provider_order = self.provider_order.write().await;
        *provider_order = order;
    }

    /// 执行请求
    pub async fn complete(
        &self,
        primitive: &PrimitiveRequest,
        _target_format: Format,
    ) -> Result<LlmResponse, GatewayError> {
        // 获取 Provider 顺序
        let provider_ids = {
            let order = self.provider_order.read().await;
            order.clone()
        };

        let mut last_error: Option<GatewayError> = None;

        for provider_id in provider_ids {
            match self.try_provider(&provider_id, primitive).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // 检查是否应该降级
                    let fallback = match &e {
                        GatewayError::ProviderError { should_fallback, .. } => *should_fallback,
                        _ => false,
                    };
                    if fallback && self.config.enable_fallback {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        // 所有 Provider 都尝试过，返回最后一个错误
        Err(last_error.unwrap_or(GatewayError::NoProviderAvailable))
    }

    /// 尝试使用指定 Provider 执行请求
    async fn try_provider(
        &self,
        provider_id: &str,
        primitive: &PrimitiveRequest,
    ) -> Result<LlmResponse, GatewayError> {
        // 全局限流
        self.global_bucket.acquire().await;

        // Provider 限流
        {
            let buckets = self.provider_buckets.read().await;
            if let Some(bucket) = buckets.get(provider_id) {
                bucket.acquire().await;
            }
        }

        // 获取 Provider
        let provider = {
            let providers = self.providers.read().await;
            providers
                .get(provider_id)
                .cloned()
                .ok_or(GatewayError::ProviderNotFound(provider_id.to_string()))?
        };

        // 编译请求
        let body = provider.compile(primitive);

        // 执行请求（带重试）
        let mut retries = 0;
        loop {
            match provider.complete(body.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // 构造 ProviderError
                    let provider_error = ProviderError {
                        message: e.to_string(),
                        retryable: retries < self.config.max_retries,
                        should_fallback: true,
                        retry_after_ms: None,
                    };

                    // 检查是否可重试
                    if provider_error.retryable && retries < self.config.max_retries {
                        retries += 1;
                        let delay = self.calculate_retry_delay(retries, &provider_error);
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    // 转换为 GatewayError
                    return Err(GatewayError::ProviderError {
                        provider_id: provider_id.to_string(),
                        message: provider_error.message,
                        should_fallback: provider_error.should_fallback,
                    });
                }
            }
        }
    }

    /// 计算重试延迟
    fn calculate_retry_delay(&self, attempt: u32, error: &ProviderError) -> Duration {
        if let Some(retry_after) = error.retry_after_ms {
            Duration::from_millis(retry_after)
        } else {
            let base = self.config.retry_base_delay_ms;
            let delay = base * (1 << (attempt - 1)); // 指数退避
            Duration::from_millis(delay.min(30_000)) // 最大 30 秒
        }
    }

    /// 获取所有已注册的 Provider ID
    pub async fn list_providers(&self) -> Vec<String> {
        let order = self.provider_order.read().await;
        order.clone()
    }
}

/// Gateway 错误
#[derive(Debug, Clone)]
pub enum GatewayError {
    /// Provider 未找到
    ProviderNotFound(String),
    /// 没有可用的 Provider
    NoProviderAvailable,
    /// Provider 执行错误
    ProviderError {
        provider_id: String,
        message: String,
        should_fallback: bool,
    },
    /// 超时
    Timeout,
    /// 限流
    RateLimited,
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            GatewayError::NoProviderAvailable => write!(f, "No provider available"),
            GatewayError::ProviderError { provider_id, message, .. } => {
                write!(f, "Provider {} error: {}", provider_id, message)
            }
            GatewayError::Timeout => write!(f, "Request timeout"),
            GatewayError::RateLimited => write!(f, "Rate limited"),
        }
    }
}

impl std::error::Error for GatewayError {}
