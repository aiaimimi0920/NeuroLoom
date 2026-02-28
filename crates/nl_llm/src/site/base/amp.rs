use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Amp 平台共享配置
///
/// 由 `AmpSite` 和 `AmpExtension` 共享，确保 base_url 和 provider 始终一致。
#[derive(Clone, Debug)]
pub struct AmpConfig {
    /// 基础 URL，默认 https://ampcode.com
    pub base_url: String,
    /// 后端供应商名称（用于 URL 路由），默认 "openai"
    pub provider: String,
}

impl AmpConfig {
    pub fn new() -> Self {
        Self {
            base_url: "https://ampcode.com".to_string(),
            provider: "openai".to_string(),
        }
    }

    /// 构建 chat completions URL
    pub fn build_chat_url(&self) -> String {
        format!(
            "{}/api/provider/{}/v1/chat/completions",
            self.base_url.trim_end_matches('/'),
            self.provider
        )
    }

    /// 构建 models URL
    pub fn build_models_url(&self) -> String {
        format!(
            "{}/api/provider/{}/v1/models",
            self.base_url.trim_end_matches('/'),
            self.provider
        )
    }

    /// 构建 embeddings URL
    pub fn build_embeddings_url(&self) -> String {
        format!(
            "{}/api/provider/{}/v1/embeddings",
            self.base_url.trim_end_matches('/'),
            self.provider
        )
    }

    /// 构建 images URL
    pub fn build_images_url(&self) -> String {
        format!(
            "{}/api/provider/{}/v1/images/generations",
            self.base_url.trim_end_matches('/'),
            self.provider
        )
    }
}

impl Default for AmpConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Sourcegraph Amp 站点 (ampcode.com)
///
/// Sourcegraph Amp 是一个 AI 编码助手平台，提供 OpenAI 兼容的供应商路由接口。
/// 通过 `/api/provider/{provider}/v1/chat/completions` 路径模式
/// 将请求路由到不同的后端供应商（OpenAI / Anthropic / Google 等）。
///
/// ## 默认配置
///
/// - 端点：`https://ampcode.com`
/// - 认证：`Authorization: Bearer <AMP_API_KEY>`
/// - 超时：180 秒（代理链路延迟较高）
///
/// ## URL 路由结构
///
/// | 操作 | URL 路径 |
/// |------|----------|
/// | 对话 | `/api/provider/{provider}/v1/chat/completions` |
/// | 流式 | `/api/provider/{provider}/v1/chat/completions` |
/// | 嵌入 | `/api/provider/{provider}/v1/embeddings` |
/// | 图像 | `/api/provider/{provider}/v1/images/generations` |
///
/// ## 示例
///
/// ```
/// // 使用默认配置
/// let site = AmpSite::new();
///
/// // 共享配置（确保 Site 和 Extension URL 一致）
/// let config = Arc::new(AmpConfig::new());
/// let site = AmpSite::from_config(config.clone());
/// let ext = AmpExtension::from_config(config);
/// ```
pub struct AmpSite {
    config: Arc<AmpConfig>,
    timeout: Duration,
}

impl AmpSite {
    pub fn new() -> Self {
        Self {
            config: Arc::new(AmpConfig::new()),
            timeout: Duration::from_secs(180),
        }
    }

    /// 从共享配置创建（确保与 AmpExtension 使用同一份配置）
    pub fn from_config(config: Arc<AmpConfig>) -> Self {
        Self {
            config,
            timeout: Duration::from_secs(180),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.config).base_url = url.into();
        self
    }

    /// 设置供应商路由名称
    ///
    /// Amp 会将请求路由到指定的后端供应商。不同的供应商提供不同的模型和服务。
    ///
    /// # 常用供应商
    ///
    /// | provider | 后端 | 可用模型示例 |
    /// |----------|------|-------------|
    /// | `"openai"` | OpenAI | gpt-4o, o1, o3-mini |
    /// | `"anthropic"` | Anthropic | claude-sonnet-4-20250514, claude-opus-4-20250514 |
    /// | `"google"` | Google | gemini-2.5-pro, gemini-2.5-flash |
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use nl_llm::site::base::amp::AmpSite;
    ///
    /// // 使用 Claude 后端
    /// let site = AmpSite::new()
    ///     .with_provider("anthropic");
    ///
    /// // 使用 Gemini 后端
    /// let site = AmpSite::new()
    ///     .with_provider("google");
    ///
    /// // 共享配置场景
    /// let config = Arc::new(nl_llm::site::base::amp::AmpConfig::new());
    /// Arc::make_mut(&mut config.clone()).provider = "anthropic".to_string();
    /// let site = AmpSite::from_config(config);
    /// ```
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.config).provider = provider.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 获取共享配置的引用
    pub fn config(&self) -> &Arc<AmpConfig> {
        &self.config
    }
}

impl Default for AmpSite {
    fn default() -> Self {
        Self::new()
    }
}

impl Site for AmpSite {
    fn id(&self) -> &str {
        "amp"
    }

    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        match ctx.action {
            Action::Generate | Action::Stream => self.config.build_chat_url(),
            Action::Embed => self.config.build_embeddings_url(),
            Action::ImageGenerate => self.config.build_images_url(),
        }
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
