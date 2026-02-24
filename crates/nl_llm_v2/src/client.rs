use std::sync::Arc;
use tokio::sync::Mutex;
use reqwest::Client;

use crate::site::Site;
use crate::auth::Authenticator;
use crate::auth::providers::{ApiKeyAuth, IFlowAuth, ServiceAccountAuth, AnthropicApiKeyAuth};
use crate::protocol::traits::{ProtocolFormat, ProtocolHook};
use crate::model::{ModelResolver, DefaultModelResolver, Capability};
use crate::pipeline::{Pipeline, PipelineContext};
use crate::pipeline::stages::{PrimitivizeStage, PackStage, AuthenticateStage, SendStage, UnpackStage};
use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, BoxLlmStream};
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::site::context::{UrlContext, Action};

/// LLM 客户端门面类
pub struct LlmClient {
    site: Arc<dyn Site>,
    authenticator: Arc<Mutex<Box<dyn Authenticator>>>,
    protocol: Arc<dyn ProtocolFormat>,
    protocol_hooks: Vec<Arc<dyn ProtocolHook>>, 
    model_resolver: Box<dyn ModelResolver>,
    default_model: String,
    http: Client,
    extension: Option<Arc<dyn ProviderExtension>>,
}

impl LlmClient {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// 从预设创建配置
    pub fn from_preset(name: &str) -> Option<ClientBuilder> {
        crate::presets::registry::REGISTRY.get_builder(name)
    }

    /// 模型别名解析
    pub fn resolve_model(&self, model: &str) -> String {
        self.model_resolver.resolve(model)
    }

    /// 检查模型能力
    pub fn has_capability(&self, model: &str, cap: Capability) -> bool {
        self.model_resolver.has_capability(model, cap)
    }

    /// 获取上下文窗口建议
    pub fn context_window_hint(&self, model: &str) -> (usize, usize) {
        self.model_resolver.context_window_hint(model)
    }

    /// 拼装处理管线
    fn build_pipeline(&self, _is_stream: bool) -> Pipeline {
        let mut pipeline = Pipeline::new();
        pipeline.add_stage(Box::new(PrimitivizeStage {}));
        pipeline.add_stage(Box::new(AuthenticateStage::new(self.authenticator.clone())));
        pipeline.add_stage(Box::new(PackStage::new(
            self.protocol.clone(),
            self.protocol_hooks.clone(),
        )));
        // [修复] 传入 protocol 引用和 hooks，用于错误规范化和钩子调用
        pipeline.add_stage(Box::new(SendStage::with_hooks(
            self.site.clone(),
            self.authenticator.clone(),
            self.http.clone(),
            self.protocol.clone(),
            self.protocol_hooks.clone(),
        )));
        pipeline.add_stage(Box::new(UnpackStage::new(
            self.protocol.clone(),
            self.protocol_hooks.clone(),
        )));
        pipeline
    }

    /// 执行请求
    pub async fn complete(&self, req: &PrimitiveRequest) -> anyhow::Result<LlmResponse> {
        // [修复] 使用 default_model 作为 fallback
        // 原因：当请求未指定模型时，应使用客户端配置的默认模型
        let model = if req.model.is_empty() {
            &self.default_model
        } else {
            &req.model
        };

        let pipeline = self.build_pipeline(false);

        // [修复] 从 authenticator 获取正确的 auth_type，而非硬编码
        // 原因：不同认证方式会影响 URL 构建（如 Vertex AI 的 SA vs API Key）
        let auth = self.authenticator.lock().await;
        let auth_type = auth.auth_type();
        drop(auth);  // 提前释放锁，避免后续阶段死锁

        let url_context = UrlContext {
            model,
            auth_type,
            action: Action::Generate,
            tenant: None,
        };
        let mut ctx = PipelineContext::from_primitive(req.clone(), url_context);

        pipeline.execute(&mut ctx).await?;
        ctx.take_response()
    }

    /// 执行流式聊天
    pub async fn stream(&self, req: &PrimitiveRequest) -> anyhow::Result<BoxLlmStream> {
        // [修复] 使用 default_model 作为 fallback
        let model = if req.model.is_empty() {
            &self.default_model
        } else {
            &req.model
        };

        let mut req_stream = req.clone();
        req_stream.stream = true;
        let pipeline = self.build_pipeline(true);

        // [修复] 从 authenticator 获取正确的 auth_type
        let auth = self.authenticator.lock().await;
        let auth_type = auth.auth_type();
        drop(auth);

        let url_context = UrlContext {
            model,
            auth_type,
            action: Action::Stream,
            tenant: None,
        };
        let mut ctx = PipelineContext::from_primitive(req_stream, url_context);

        pipeline.execute(&mut ctx).await?;
        ctx.take_stream()
    }

    /// 获取底层 Authenticator
    pub fn authenticator(&self) -> Arc<Mutex<Box<dyn Authenticator>>> {
        self.authenticator.clone()
    }

    /// 获取可用模型列表（如果平台支持该扩展）
    pub async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        if let Some(ext) = &self.extension {
            let mut auth = self.authenticator.lock().await;
            ext.list_models(&self.http, &mut **auth).await
        } else {
            Err(anyhow::anyhow!("Extension API (list_models) not supported for this provider"))
        }
    }

    /// 获取账户额度/余额信息（如果平台支持该扩展）
    pub async fn get_balance(&self) -> anyhow::Result<Option<String>> {
        if let Some(ext) = &self.extension {
            let mut auth = self.authenticator.lock().await;
            ext.get_balance(&self.http, &mut **auth).await
        } else {
            Err(anyhow::anyhow!("Extension API (get_balance) not supported for this provider"))
        }
    }
}

/// 客户端构建器
pub struct ClientBuilder {
    site: Option<Arc<dyn Site>>,
    authenticator: Option<Box<dyn Authenticator>>,
    protocol: Option<Arc<dyn ProtocolFormat>>,
    protocol_hooks: Vec<Arc<dyn ProtocolHook>>,
    model_resolver: Option<Box<dyn ModelResolver>>,
    default_model: Option<String>,
    http: Option<Client>,
    extension: Option<Arc<dyn ProviderExtension>>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            site: None,
            authenticator: None,
            protocol: None,
            protocol_hooks: Vec::new(),
            model_resolver: None,
            default_model: None,
            http: None,
            extension: None,
        }
    }

    pub fn site(mut self, site: impl Site + 'static) -> Self {
        self.site = Some(Arc::new(site));
        self
    }

    pub fn auth(mut self, auth: impl Authenticator + 'static) -> Self {
        self.authenticator = Some(Box::new(auth));
        self
    }

    pub fn with_api_key(self, key: impl Into<String>) -> Self {
        self.auth(ApiKeyAuth::new(key))
    }

    pub fn with_cookie(self, cookie: impl Into<String>) -> Self {
        self.auth(IFlowAuth::new(cookie))
    }

    pub fn with_service_account_json(self, json_str: impl Into<String>) -> Self {
        self.auth(ServiceAccountAuth::new(json_str))
    }

    /// Gemini CLI 专用：使用本地 OAuth 缓存文件或自动执行浏览器授权
    pub fn with_gemini_cli_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::gemini_cli::GeminiCliOAuth::new().with_cache(cache_path))
    }

    /// Antigravity 专用：使用专属的 Client ID / Secret 和广度 Scopes 登录
    pub fn with_antigravity_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::antigravity::AntigravityOAuth::new().with_cache(cache_path))
    }

    /// Anthropic 专用认证（使用 x-api-key header）
    pub fn with_anthropic_api_key(self, key: impl Into<String>) -> Self {
        self.auth(AnthropicApiKeyAuth::new(key))
    }

    /// 修改 Base URL（代理站场景）
    /// 用于代理站场景：LlmClient::from_preset("openai").with_base_url("https://proxy.example.com/v1")
    /// [修复] 使用 ProxySite 包装原站点，保留 URL 构建逻辑
    /// 原因：代理站需要保持原有 URL 路径结构，只替换 base URL
    /// 注意：必须在设置了 site 或使用 from_preset 之后调用
    pub fn with_base_url(self, url: impl Into<String>) -> Self {
        let url = url.into();
        match self.site {
            Some(inner) => Self {
                site: Some(Arc::new(crate::site::base::proxy::ProxySite::new(inner, url))),
                ..self
            },
            None => {
                // 如果还没设置 site，使用 SimpleSite 作为基础
                use crate::site::traits::SimpleSite;
                use std::time::Duration;
                Self {
                    site: Some(Arc::new(SimpleSite {
                        id: "proxy".to_string(),
                        base_url: url,
                        extra_headers: std::collections::HashMap::new(),
                        timeout: Duration::from_secs(60),
                    })),
                    ..self
                }
            }
        }
    }

    /// 设置默认模型的便捷别名
    pub fn with_model(self, model: impl Into<String>) -> Self {
        self.default_model(model)
    }

    pub fn protocol(mut self, protocol: impl ProtocolFormat + 'static) -> Self {
        self.protocol = Some(Arc::new(protocol));
        self
    }

    pub fn with_protocol_hook(mut self, hook: Arc<dyn ProtocolHook>) -> Self {
        self.protocol_hooks.push(hook);
        self
    }

    /// 设置供应商扩展接口
    pub fn with_extension(mut self, ext: Arc<dyn ProviderExtension>) -> Self {
        self.extension = Some(ext);
        self
    }

    pub fn model_resolver(mut self, resolver: impl ModelResolver + 'static) -> Self {
        self.model_resolver = Some(Box::new(resolver));
        self
    }

    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    pub fn build(self) -> LlmClient {
        LlmClient {
            site: self.site.expect("site is required"),
            authenticator: Arc::new(Mutex::new(
                self.authenticator.expect("auth is required")
            )),
            protocol: self.protocol.expect("protocol is required"),
            protocol_hooks: self.protocol_hooks,
            model_resolver: self.model_resolver
                .unwrap_or_else(|| Box::new(DefaultModelResolver::new())),
            default_model: self.default_model.unwrap_or_default(),
            http: self.http.unwrap_or_else(Client::new),
            extension: self.extension,
        }
    }
}
