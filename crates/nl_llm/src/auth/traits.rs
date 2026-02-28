use async_trait::async_trait;
use reqwest::RequestBuilder;

use crate::site::context::AuthType;

/// 认证器定义
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// 认证器标识
    fn id(&self) -> &str;

    /// 是否已认证
    fn is_authenticated(&self) -> bool;

    /// 是否需要刷新
    fn needs_refresh(&self) -> bool {
        false
    }

    /// 刷新认证（异步）
    async fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// 注入认证信息到请求
    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder>;

    /// 获取认证类型（用于 URL 构建）
    fn auth_type(&self) -> AuthType;

    /// 获取额外的元数据（例如 OAuth 登录获取到的 Project ID 等）
    fn get_extra<'a>(&'a self) -> Option<&'a std::collections::HashMap<String, serde_json::Value>> {
        None
    }
}
