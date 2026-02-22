//! 降级路由

use std::collections::HashMap;

/// 降级配置
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// 默认降级链
    pub default_chain: Vec<String>,
    /// 按 Provider 定制的降级链
    pub provider_chains: HashMap<String, Vec<String>>,
    /// 是否在降级时记录日志
    pub log_fallback: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            default_chain: vec![
                "claude".to_string(),
                "openai".to_string(),
                "gemini".to_string(),
            ],
            provider_chains: HashMap::new(),
            log_fallback: true,
        }
    }
}

/// 降级路由器
pub struct FallbackRouter {
    config: FallbackConfig,
}

impl FallbackRouter {
    /// 创建新的降级路由器
    pub fn new(config: FallbackConfig) -> Self {
        Self { config }
    }

    /// 获取指定 Provider 的降级链
    pub fn get_fallback_chain(&self, provider_id: &str) -> Vec<String> {
        // 先查找定制降级链
        if let Some(chain) = self.config.provider_chains.get(provider_id) {
            return chain.clone();
        }

        // 返回默认降级链（排除当前 Provider）
        self.config
            .default_chain
            .iter()
            .filter(|id| *id != provider_id)
            .cloned()
            .collect()
    }

    /// 设置默认降级链
    pub fn set_default_chain(&mut self, chain: Vec<String>) {
        self.config.default_chain = chain;
    }

    /// 设置指定 Provider 的降级链
    pub fn set_provider_chain(&mut self, provider_id: String, chain: Vec<String>) {
        self.config.provider_chains.insert(provider_id, chain);
    }

    /// 是否启用日志
    pub fn should_log(&self) -> bool {
        self.config.log_fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fallback_chain() {
        let router = FallbackRouter::new(FallbackConfig::default());

        let chain = router.get_fallback_chain("claude");

        // 应该返回默认链中排除 claude 的部分
        assert!(!chain.contains(&"claude".to_string()));
        assert!(chain.contains(&"openai".to_string()));
        assert!(chain.contains(&"gemini".to_string()));
    }

    #[test]
    fn test_custom_fallback_chain() {
        let mut config = FallbackConfig::default();
        config.provider_chains.insert(
            "claude".to_string(),
            vec!["gemini".to_string(), "openai".to_string()],
        );

        let router = FallbackRouter::new(config);

        let chain = router.get_fallback_chain("claude");

        // 应该返回定制链
        assert_eq!(chain, vec!["gemini", "openai"]);
    }
}
