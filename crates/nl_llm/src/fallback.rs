//! 断流重试与降级路由

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// LLM 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 提供商名称
    pub name: String,
    /// API 端点
    pub endpoint: String,
    /// API 密钥 (可选)
    pub api_key: Option<String>,
    /// 模型名称
    pub model: String,
    /// 优先级 (越小越优先)
    pub priority: u8,
    /// 最大重试次数
    pub max_retries: u32,
    /// 超时时间
    pub timeout: Duration,
    /// 是否启用
    pub enabled: bool,
}

/// 降级路由器
pub struct FallbackRouter {
    /// 提供商列表 (按优先级排序)
    providers: Vec<ProviderConfig>,
    /// 当前活跃提供商索引
    current_index: usize,
}

impl FallbackRouter {
    /// 创建新的降级路由器
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        let mut providers = providers;
        providers.sort_by_key(|p| p.priority);
        Self {
            providers,
            current_index: 0,
        }
    }

    /// 创建默认配置
    pub fn default_config() -> Self {
        Self::new(vec![
            ProviderConfig {
                name: "anthropic".to_string(),
                endpoint: "https://api.anthropic.com".to_string(),
                api_key: None,
                model: "claude-3-5-sonnet".to_string(),
                priority: 0,
                max_retries: 3,
                timeout: Duration::from_secs(60),
                enabled: true,
            },
            ProviderConfig {
                name: "openai".to_string(),
                endpoint: "https://api.openai.com".to_string(),
                api_key: None,
                model: "gpt-4".to_string(),
                priority: 1,
                max_retries: 3,
                timeout: Duration::from_secs(60),
                enabled: true,
            },
            ProviderConfig {
                name: "ollama".to_string(),
                endpoint: "http://localhost:11434".to_string(),
                api_key: None,
                model: "llama3".to_string(),
                priority: 2,
                max_retries: 1,
                timeout: Duration::from_secs(120),
                enabled: true,
            },
        ])
    }

    /// 获取当前提供商
    pub fn current(&self) -> Option<&ProviderConfig> {
        self.providers.get(self.current_index)
    }

    /// 切换到下一个提供商 (降级)
    pub fn fallback(&mut self) -> Option<&ProviderConfig> {
        for (i, provider) in self.providers.iter().enumerate().skip(self.current_index + 1) {
            if provider.enabled {
                self.current_index = i;
                return Some(provider);
            }
        }
        None
    }

    /// 重置到首选提供商
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// 获取所有提供商
    pub fn providers(&self) -> &[ProviderConfig] {
        &self.providers
    }
}
