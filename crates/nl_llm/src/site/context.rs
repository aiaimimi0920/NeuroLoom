/// 认证类型枚举（用于 URL 构建）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    ApiKey,
    OAuth,
    ServiceAccount,
    Cookie,
    MultiKey,
}

/// 操作类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 普通生成
    Generate,
    /// 流式生成
    Stream,
    /// 向量嵌入
    Embed,
    /// 图像生成
    ImageGenerate,
}

/// URL 构建上下文
pub struct UrlContext<'a> {
    /// 模型名称
    pub model: &'a str,
    /// 认证类型
    pub auth_type: AuthType,
    /// 操作类型
    pub action: Action,
    /// 租户信息（多租户场景）
    pub tenant: Option<TenantInfo>,
}

/// 租户信息
pub struct TenantInfo {
    pub tenant_id: String,
    pub project_id: Option<String>,
}
