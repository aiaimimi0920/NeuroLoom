use crate::site::Site;
use std::time::Duration;

/// 腾讯云 TI 平台大模型站点 (Tencent Cloud API V3)
#[derive(Debug, Clone)]
pub struct TencentTiSite {
    base_url: String,
}

impl Default for TencentTiSite {
    fn default() -> Self {
        Self::new()
    }
}

impl TencentTiSite {
    pub fn new() -> Self {
        Self {
            // 腾讯云 TI 平台混元大模型 API 接入点
            base_url: "https://hunyuan.tencentcloudapi.com".to_string(),
        }
    }
}

impl Site for TencentTiSite {
    fn id(&self) -> &str {
        "tencent_ti"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &crate::site::context::UrlContext) -> String {
        // Tencent API V3 的请求地址通常就是根路径 "/", 通过 Host 和 Action header 路由
        self.base_url.clone()
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type", "application/json");
        // 注意：Tencent API V3 其他头如 X-TC-Action 由 Authenticator 注入
        headers
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }
}
