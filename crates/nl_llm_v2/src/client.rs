use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::auth::providers::{
    AnthropicApiKeyAuth, ApiKeyAuth, IFlowAuth, ServiceAccountAuth, SparkAuth,
};
use crate::auth::Authenticator;
use crate::concurrency::{ConcurrencyConfig, ConcurrencyController, FailureType};
use crate::metrics::{MetricsStore, MetricsSummary, PipelineMetrics};
use crate::model::{Capability, DefaultModelResolver, ModelResolver};
use crate::pipeline::stages::{
    AuthenticateStage, PackStage, PrimitivizeStage, SendStage, UnpackStage,
};
use crate::pipeline::{Pipeline, PipelineContext};
use crate::primitive::PrimitiveRequest;
use crate::protocol::traits::{ProtocolFormat, ProtocolHook};
use crate::provider::balance::BalanceStatus;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::{BoxLlmStream, LlmResponse};
use crate::site::context::{Action, UrlContext};
use crate::site::Site;

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
    /// 并发控制器
    concurrency: Option<Arc<ConcurrencyController>>,
    /// 指标存储
    metrics: MetricsStore,
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

    /// 获取模型最大上下文长度
    pub fn max_context(&self, model: &str) -> usize {
        self.model_resolver.max_context(model)
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
        // 创建指标记录
        let mut metrics = PipelineMetrics::new();

        // 获取并发许可证（如果启用了并发控制）
        let permit = if let Some(ctrl) = &self.concurrency {
            Some(ctrl.acquire().await)
        } else {
            None
        };

        // [修复] 使用 default_model 作为 fallback，并 resolve 别名
        // 原因：模型别名（如 "codex"）需要解析为实际模型名（如 "gpt-5.1-codex"）
        let model_raw = if req.model.is_empty() {
            &self.default_model
        } else {
            &req.model
        };
        let resolved_model = self.model_resolver.resolve(model_raw);

        let pipeline = self.build_pipeline(false);

        // [修复] 从 authenticator 获取正确的 auth_type，而非硬编码
        // 原因：不同认证方式会影响 URL 构建（如 Vertex AI 的 SA vs API Key）
        let auth = self.authenticator.lock().await;
        let auth_type = auth.auth_type();
        drop(auth); // 提前释放锁，避免后续阶段死锁

        let url_context = UrlContext {
            model: &resolved_model,
            auth_type,
            action: Action::Generate,
            tenant: None,
        };

        // [修复] 将 resolved 后的模型名写入 primitive，确保 protocol.pack() 使用正确的模型名
        let mut resolved_req = req.clone();
        resolved_req.model = resolved_model.clone();

        let mut ctx = PipelineContext::from_primitive(resolved_req, url_context);

        let result = pipeline.execute(&mut ctx).await;

        // 记录指标并报告并发状态
        match &result {
            Ok(_) => {
                metrics.finish();
                self.metrics.record(metrics);
                if let Some(permit) = permit {
                    permit.report_success();
                }
            }
            Err(e) => {
                metrics.finish();
                self.metrics.record(metrics.finish_error(&e.to_string()));
                if let Some(permit) = permit {
                    // 根据错误类型判断失败类型
                    let failure_type = classify_error(e);
                    permit.report_failure(failure_type);
                }
            }
        }

        result?;
        ctx.take_response()
    }

    /// 执行流式聊天
    pub async fn stream(&self, req: &PrimitiveRequest) -> anyhow::Result<BoxLlmStream> {
        // 创建指标记录
        let mut metrics = PipelineMetrics::new();

        // 获取并发许可证（如果启用了并发控制）
        let permit = if let Some(ctrl) = &self.concurrency {
            Some(ctrl.acquire().await)
        } else {
            None
        };

        // [修复] 使用 default_model 作为 fallback，并 resolve 别名
        let model_raw = if req.model.is_empty() {
            &self.default_model
        } else {
            &req.model
        };
        let resolved_model = self.model_resolver.resolve(model_raw);

        let mut req_stream = req.clone();
        req_stream.stream = true;
        req_stream.model = resolved_model.clone();
        let pipeline = self.build_pipeline(true);

        // [修复] 从 authenticator 获取正确的 auth_type
        let auth = self.authenticator.lock().await;
        let auth_type = auth.auth_type();
        drop(auth);

        let url_context = UrlContext {
            model: &resolved_model,
            auth_type,
            action: Action::Stream,
            tenant: None,
        };
        let mut ctx = PipelineContext::from_primitive(req_stream, url_context);

        let result = pipeline.execute(&mut ctx).await;

        // 记录指标并报告并发状态
        match &result {
            Ok(_) => {
                metrics.finish();
                self.metrics.record(metrics);
                if let Some(permit) = permit {
                    permit.report_success();
                }
            }
            Err(e) => {
                metrics.finish();
                self.metrics.record(metrics.finish_error(&e.to_string()));
                if let Some(permit) = permit {
                    let failure_type = classify_error(e);
                    permit.report_failure(failure_type);
                }
            }
        }

        result?;
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
            Err(anyhow::anyhow!(
                "Extension API (list_models) not supported for this provider"
            ))
        }
    }

    /// 获取账户额度/余额信息（如果平台支持该扩展）
    pub async fn get_balance(&self) -> anyhow::Result<Option<BalanceStatus>> {
        if let Some(ext) = &self.extension {
            let mut auth = self.authenticator.lock().await;
            ext.get_balance(&self.http, &mut **auth).await
        } else {
            Err(anyhow::anyhow!(
                "Extension API (get_balance) not supported for this provider"
            ))
        }
    }

    /// 提交异步视频生成任务（如可灵 Kling、Luma 等）
    pub async fn submit_video_task(&self, req: &PrimitiveRequest) -> anyhow::Result<String> {
        if let Some(ext) = &self.extension {
            // 与 complete()/stream() 保持一致：视频任务同样需要做模型别名解析。
            // 例如 Jimeng: "jimeng-v3.0" -> "jimeng_t2v_v30" (req_key)
            let model_raw = if req.model.is_empty() {
                &self.default_model
            } else {
                &req.model
            };
            let resolved_model = self.model_resolver.resolve(model_raw);

            let mut resolved_req = req.clone();
            resolved_req.model = resolved_model;

            let mut auth = self.authenticator.lock().await;
            ext.submit_video_task(&self.http, &mut **auth, &resolved_req).await
        } else {
            Err(anyhow::anyhow!(
                "Extension API (submit_video_task) not supported for this provider"
            ))
        }
    }

    /// 查询异步视频生成任务状态
    pub async fn fetch_video_task(&self, task_id: &str) -> anyhow::Result<crate::provider::extension::VideoTaskStatus> {
        if let Some(ext) = &self.extension {
            let mut auth = self.authenticator.lock().await;
            ext.fetch_video_task(&self.http, &mut **auth, task_id).await
        } else {
            Err(anyhow::anyhow!(
                "Extension API (fetch_video_task) not supported for this provider"
            ))
        }
    }

    /// 获取并发控制器引用
    pub fn concurrency_controller(&self) -> Option<&ConcurrencyController> {
        self.concurrency.as_deref()
    }

    /// 获取并发状态快照
    pub fn concurrency_snapshot(&self) -> Option<crate::concurrency::ConcurrencySnapshot> {
        self.concurrency.as_ref().map(|c| c.snapshot())
    }

    /// 获取指标摘要
    pub fn metrics_summary(&self) -> MetricsSummary {
        self.metrics.summary()
    }

    /// 获取指标存储引用
    pub fn metrics_store(&self) -> &MetricsStore {
        &self.metrics
    }
}

/// 根据错误类型分类
fn classify_error(e: &anyhow::Error) -> FailureType {
    let err_str = e.to_string().to_lowercase();

    // 检查是否是 429 限流错误
    if err_str.contains("429")
        || err_str.contains("rate limit")
        || err_str.contains("too many requests")
    {
        return FailureType::RateLimited;
    }

    // 检查是否是超时错误
    if err_str.contains("timeout") || err_str.contains("timed out") {
        return FailureType::Timeout;
    }

    // 检查是否是服务端错误
    if err_str.contains("500")
        || err_str.contains("502")
        || err_str.contains("503")
        || err_str.contains("server error")
    {
        return FailureType::ServerError;
    }

    FailureType::Other
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
    /// 并发控制配置（None 表示不启用并发控制）
    concurrency_config: Option<ConcurrencyConfig>,
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
            concurrency_config: None,
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

    /// 讯飞星火专用认证。
    ///
    /// 支持 `APIPassword`（推荐）和 `APIKey:APISecret`（兼容）两种输入。
    pub fn with_spark_auth(self, token: impl Into<String>) -> Self {
        self.auth(SparkAuth::new(token))
    }

    pub fn with_cookie(self, cookie: impl Into<String>) -> Self {
        self.auth(IFlowAuth::new(cookie))
    }

    /// iFlow 专用认证（支持缓存以避免风控封禁）
    pub fn with_iflow_cookie(self, cookie: impl Into<String>, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(IFlowAuth::new(cookie).with_cache(cache_path))
    }

    /// 启用并发控制（使用 Extension 中的默认配置或默认值）
    pub fn with_concurrency(mut self) -> Self {
        // 如果有 Extension，使用其并发配置；否则使用默认配置
        self.concurrency_config = Some(
            self.extension
                .as_ref()
                .map(|ext| ext.concurrency_config())
                .unwrap_or_default(),
        );
        self
    }

    /// 启用并发控制（使用自定义配置）
    pub fn with_concurrency_config(mut self, config: ConcurrencyConfig) -> Self {
        self.concurrency_config = Some(config);
        self
    }

    pub fn with_service_account_json(mut self, json_str: impl Into<String>) -> Self {
        let json_str = json_str.into();

        // 从 SA JSON 中提取 project_id 用于构建 VertexSite URL
        #[derive(serde::Deserialize)]
        struct SaProjectInfo {
            project_id: Option<String>,
        }

        let project_id = serde_json::from_str::<SaProjectInfo>(&json_str)
            .ok()
            .and_then(|sa| sa.project_id)
            .unwrap_or_else(|| "UNKNOWN_PROJECT".to_string());

        // 重建 VertexSite 使用真实的 project_id
        self.site = Some(Arc::new(crate::site::base::vertex::VertexSite::new(
            &project_id,
            "us-central1",
        )));

        // 注入 VertexExtension（需要 project_id 和 location 来调用真实 API）
        self.extension = Some(Arc::new(crate::provider::vertex::VertexExtension::new(
            &project_id,
            "us-central1",
        )));

        self.auth(ServiceAccountAuth::new(json_str))
    }

    /// Vertex AI (API Key 模式) 专用
    ///
    /// API Key 通过 URL `?key=xxx` 注入。
    /// 注意：API Key 模式的 URL 不含 project_id（直接走 publishers/google/models/）。
    /// ```
    /// let client = LlmClient::from_preset("vertex_api")
    ///     .with_vertex_api_key("AIza...")
    ///     .build();
    /// ```
    pub fn with_vertex_api_key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();

        // VertexApiSite: API Key 模式不需要 project_id
        self.site = Some(Arc::new(crate::site::base::vertex_api::VertexApiSite::new(
            &key,
        )));

        // API Key 模式使用 GeminiExtension（走 generativelanguage.googleapis.com，支持 API Key）
        // 而非 VertexExtension（走 aiplatform.googleapis.com，需要 Bearer Token）
        self.extension = Some(Arc::new(
            crate::provider::gemini::GeminiExtension::new().with_api_key(&key),
        ));

        // GeminiApiKeyAuth: 不注入 Header（key 已在 URL 中）
        self.auth(crate::auth::providers::GeminiApiKeyAuth::new(key))
    }

    /// Gemini CLI 专用：使用本地 OAuth 缓存文件或自动执行浏览器授权
    pub fn with_gemini_cli_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::gemini_cli::GeminiCliOAuth::new().with_cache(cache_path))
    }

    /// Antigravity 专用：使用专属的 Client ID / Secret 和广度 Scopes 登录
    pub fn with_antigravity_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(
            crate::auth::providers::antigravity::AntigravityOAuth::new().with_cache(cache_path),
        )
    }

    /// Qwen OAuth 专用：使用 Device Code + PKCE 浏览器授权
    ///
    /// ```
    /// let client = LlmClient::from_preset("qwen")
    ///     .with_qwen_oauth("~/.config/qwen/token.json")
    ///     .build();
    /// ```
    pub fn with_qwen_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::qwen::QwenOAuth::new().with_cache(cache_path))
    }

    /// Kimi OAuth 专用：使用 RFC 8628 Device Code 浏览器授权
    ///
    /// ```
    /// let client = LlmClient::from_preset("kimi")
    ///     .with_kimi_oauth("~/.config/kimi/token.json")
    ///     .build();
    /// ```
    pub fn with_kimi_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::kimi::KimiOAuth::new(cache_path))
    }

    /// Claude OAuth 专用：使用 Authorization Code + PKCE 浏览器授权
    ///
    /// 首次运行会打开浏览器完成授权，后续使用缓存 token。
    ///
    /// ```
    /// let client = LlmClient::from_preset("claude_oauth")
    ///     .with_claude_oauth("~/.config/anthropic/token.json")
    ///     .build();
    /// ```
    pub fn with_claude_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::anthropic_oauth::AnthropicOAuth::new(cache_path))
    }

    /// Claude API Key 专用认证（使用 x-api-key header）
    pub fn with_claude_api_key(self, key: impl Into<String>) -> Self {
        self.auth(AnthropicApiKeyAuth::new(key))
    }

    /// Codex OAuth 专用：使用 Authorization Code + PKCE 浏览器授权
    ///
    /// ```
    /// let client = LlmClient::from_preset("codex_oauth")
    ///     .with_codex_oauth("~/.config/codex/token.json")
    ///     .build();
    /// ```
    pub fn with_codex_oauth(self, cache_path: impl AsRef<std::path::Path>) -> Self {
        self.auth(crate::auth::providers::codex_oauth::CodexOAuth::new(
            cache_path,
        ))
    }

    /// Sourcegraph Amp 专用：切换后端供应商
    ///
    /// Amp 平台通过 `/api/provider/{provider}/v1/...` 路由请求到不同后端。
    /// 默认使用 "openai" 供应商，可切换为 "anthropic"、"google" 等。
    ///
    /// # 示例
    ///
    /// ```
    /// // 使用 Anthropic 后端
    /// let client = LlmClient::from_preset("amp")
    ///     .expect("Preset should exist")
    ///     .with_api_key("your-amp-api-key")
    ///     .with_amp_provider("anthropic")
    ///     .build();
    ///
    /// // 使用 Google Gemini 后端
    /// let client = LlmClient::from_preset("amp")
    ///     .expect("Preset should exist")
    ///     .with_api_key("your-amp-api-key")
    ///     .with_amp_provider("google")
    ///     .build();
    /// ```
    pub fn with_amp_provider(self, provider: impl Into<String>) -> Self {
        let provider = provider.into();
        // 更新 AmpSite 的 provider 字段
        if let Some(site) = self.site.as_ref() {
            if site.id() == "amp" {
                // 重建 AmpSite 并设置新的 provider
                let new_site = crate::site::base::amp::AmpSite::new()
                    .with_base_url(site.base_url())
                    .with_provider(&provider)
                    .with_timeout(site.timeout());
                return Self {
                    site: Some(Arc::new(new_site)),
                    ..self
                };
            }
        }
        self
    }

    /// Gemini 官方 API 专用认证（API Key 通过 URL query `?key=` 传递）
    ///
    /// 注意：Gemini API Key 不走 HTTP Header，而是拼在 URL 中。
    /// 此方法会同时将 key 传递给 GeminiSite、GeminiExtension 和创建空壳认证器。
    pub fn with_gemini_api_key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        // 重建 GeminiSite 并注入 API Key
        self.site = Some(Arc::new(
            crate::site::base::gemini::GeminiSite::new().with_api_key(&key),
        ));
        // 注入带 key 的 Extension（用于 list_models）
        self.extension = Some(Arc::new(
            crate::provider::gemini::GeminiExtension::new().with_api_key(&key),
        ));
        self.auth(crate::auth::providers::GeminiApiKeyAuth::new(key))
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
                site: Some(Arc::new(crate::site::base::proxy::ProxySite::new(
                    inner, url,
                ))),
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
        // 如果指定了并发控制但没有提供配置，尝试从 Extension 获取
        let concurrency_config = self
            .concurrency_config
            .or_else(|| self.extension.as_ref().map(|ext| ext.concurrency_config()));

        // 创建并发控制器
        let concurrency =
            concurrency_config.map(|config| Arc::new(ConcurrencyController::new(config)));

        LlmClient {
            site: self.site.expect("site is required"),
            authenticator: Arc::new(Mutex::new(self.authenticator.expect("auth is required"))),
            protocol: self.protocol.expect("protocol is required"),
            protocol_hooks: self.protocol_hooks,
            model_resolver: self
                .model_resolver
                .unwrap_or_else(|| Box::new(DefaultModelResolver::new())),
            default_model: self.default_model.unwrap_or_default(),
            http: self.http.unwrap_or_else(Client::new),
            extension: self.extension,
            concurrency,
            metrics: MetricsStore::default(),
        }
    }
}
