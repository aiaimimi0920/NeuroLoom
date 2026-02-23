use std::collections::HashMap;
use std::time::Duration;
use crate::site::context::UrlContext;

/// 站点定义
pub trait Site: Send + Sync {
    /// 站点标识
    fn id(&self) -> &str;

    /// 获取 Base URL
    fn base_url(&self) -> &str;

    /// 构建完整请求 URL
    /// ctx 包含模型、认证类型、操作类型等信息
    fn build_url(&self, ctx: &UrlContext) -> String;

    /// 获取额外 Headers
    fn extra_headers(&self) -> HashMap<&str, &str>;

    /// 获取超时设置
    fn timeout(&self) -> Duration;
}

/// 简单站点实现
pub struct SimpleSite {
    pub id: String,
    pub base_url: String,
    pub extra_headers: HashMap<String, String>,
    pub timeout: Duration,
}

impl Site for SimpleSite {
    fn id(&self) -> &str {
        &self.id
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &UrlContext) -> String {
        self.base_url.clone()
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
