use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::site::context::UrlContext;
use crate::site::traits::Site;

/// 代理站点包装器
/// ��于代理站场景：保持原站点的 URL 构建逻辑，只替换 base URL
pub struct ProxySite {
    /// 原始站点（保留 URL 构建逻辑）
    inner: Arc<dyn Site>,
    /// 覆盖的 base URL
    base_url: String,
}

impl ProxySite {
    /// 创建代理站点
    pub fn new(inner: Arc<dyn Site>, base_url: impl Into<String>) -> Self {
        Self {
            inner,
            base_url: base_url.into(),
        }
    }
}

impl Site for ProxySite {
    fn id(&self) -> &str {
        // 标识为代理站，保留原站点信息
        self.inner.id()
    }

    fn base_url(&self) -> &str {
        // 返回覆盖后的 base URL
        &self.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        // [核心逻辑] 使用原站点的 URL 构建逻辑，但替换 base URL
        // 原因：代理站通常保持相同的 URL 路径结构，只是域名不同
        let original_url = self.inner.build_url(ctx);
        let original_base = self.inner.base_url();

        // 替换 base URL 部分
        if let Some(path) = original_url.strip_prefix(original_base) {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        } else {
            // fallback: 直接返回新 URL
            original_url.replace(original_base, &self.base_url)
        }
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        // 保留原站点的 headers
        self.inner.extra_headers()
    }

    fn timeout(&self) -> Duration {
        // 保留原站点的超时设置
        self.inner.timeout()
    }
}
