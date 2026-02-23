# nl_llm v2.2 实施计划

## 概述

本文档定义了 nl_llm v2.2 四维正交分解架构的详细实施计划。

---

## Phase 1：核心 Traits 定义

### 1.1 Site Trait

**文件**: `src/site/traits.rs`

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use crate::site::context::{UrlContext, Action};

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
```

### 1.2 URL 构建上下文

**文件**: `src/site/context.rs`

```rust
/// 认证类型枚举（用于 URL 构建）
#[derive(Debug, Clone, Copy)]
pub enum AuthType {
    ApiKey,
    OAuth,
    ServiceAccount,
    Cookie,
    MultiKey,
}

/// 操作类型枚举
#[derive(Debug, Clone, Copy)]
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
```

### 1.3 Protocol Trait

**文件**: `src/protocol/traits.rs`

```rust
use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, LlmChunk};
use crate::protocol::error::StandardError;
use crate::pipeline::PipelineContext;

/// 协议格式定义
pub trait ProtocolFormat: Send + Sync {
    /// 协议标识
    fn id(&self) -> &str;

    /// 封包：PrimitiveRequest → JSON
    /// is_stream 参数用于在 JSON body 中添加 stream 标识
    fn pack(&self, primitive: &PrimitiveRequest, is_stream: bool) -> serde_json::Value;

    /// 解包响应：JSON → LlmResponse
    fn unpack_response(&self, raw: &str) -> crate::Result<LlmResponse>;

    /// 解包流式响应
    fn unpack_stream(
        &self,
        response: reqwest::Response,
    ) -> crate::Result<BoxStream<'static, crate::Result<LlmChunk>>>;

    /// 检测格式是否匹配（用于直通优化）
    fn matches_format(&self, data: &serde_json::Value) -> bool;

    /// 解包错误：将平台错误转换为标准错误
    fn unpack_error(&self, status: u16, raw: &str) -> crate::Result<StandardError>;
}

/// 协议钩子（扩展签名，可访问 PipelineContext）
pub trait ProtocolHook: Send + Sync {
    /// 封包后处理
    fn after_pack(&self, ctx: &mut PipelineContext, packed: &mut serde_json::Value) {
        // 默认空实现
    }

    /// 解包前处理
    fn before_unpack(&self, ctx: &mut PipelineContext, data: &mut serde_json::Value) {
        // 默认空实现
    }

    /// 发送前处理（可修改 headers）
    fn before_send(&self, ctx: &mut PipelineContext, req: &mut reqwest::RequestBuilder) {
        // 默认空实现
    }

    /// 接收后处理（流式响应预处理）
    fn after_receive(&self, ctx: &mut PipelineContext, resp: &mut reqwest::Response) {
        // 默认空实现
    }
}
```

### 1.4 错误规范化

**文件**: `src/protocol/error.rs`

```rust
/// 标准错误类型
pub struct StandardError {
    /// 错误类型
    pub kind: ErrorKind,
    /// 错误消息
    pub message: String,
    /// 原始错误码（如有）
    pub code: Option<String>,
    /// 是否可重试
    pub retryable: bool,
    /// 建议的降级动作
    pub fallback_hint: Option<FallbackHint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 认证错误（401/403）
    Authentication,
    /// 配额超限（429）
    RateLimit,
    /// 模型不可用
    ModelUnavailable,
    /// 上下文超长
    ContextLengthExceeded,
    /// 内容过滤
    ContentFilter,
    /// 服务端错误（500+）
    ServerError,
    /// 其他错误
    Other,
}

#[derive(Debug, Clone)]
pub enum FallbackHint {
    /// 重试当前平台
    Retry,
    /// 降级到其他平台
    FallbackTo(String),
    /// 降低模型规格
    DowngradeModel,
    /// 无建议
    None,
}
```

### 1.5 Authenticator Trait

**文件**: `src/auth/traits.rs`

```rust
use async_trait::async_trait;

/// 认证器定义
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// 认证器标识
    fn id(&self) -> &str;

    /// 是否已认证
    fn is_authenticated(&self) -> bool;

    /// 是否需要刷新
    fn needs_refresh(&self) -> bool;

    /// 刷新认证（异步）
    async fn refresh(&mut self) -> crate::Result<()>;

    /// 注入认证信息到请求
    fn inject(&self, req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder>;

    /// 获取认证类型（用于 URL 构建）
    fn auth_type(&self) -> AuthType;
}
```

---

## Phase 2：模型解析器

### 2.1 ModelResolver Trait

**文件**: `src/model/resolver.rs`

```rust
use bitflags::bitflags;

bitflags! {
    /// 模型能力标志
    pub struct Capability: u32 {
        const CHAT = 0b0001;
        const VISION = 0b0010;
        const TOOLS = 0b0100;
        const STREAMING = 0b1000;
        const THINKING = 0b10000;
        const CODE_EXECUTION = 0b100000;
    }
}

/// 模型解析器
pub trait ModelResolver: Send + Sync {
    /// 解析模型别名到实际模型名
    fn resolve(&self, model: &str) -> String;

    /// 检查模型是否支持指定能力
    fn has_capability(&self, model: &str, cap: Capability) -> bool;

    /// 获取模型的最大上下文长度
    fn max_context(&self, model: &str) -> usize;

    /// 获取模型的上下文窗口建议（输入/输出分配）
    fn context_window_hint(&self, model: &str) -> (usize, usize);
}
```

### 2.2 默认实现

**文件**: `src/model/default.rs`

```rust
use super::resolver::{ModelResolver, Capability};
use std::collections::HashMap;

pub struct DefaultModelResolver {
    aliases: HashMap<String, String>,
    capabilities: HashMap<String, Capability>,
    context_lengths: HashMap<String, usize>,
}

impl DefaultModelResolver {
    pub fn new() -> Self {
        // 初始化别名、能力表、上下文长度表
        // ... 见设计规范文档
    }
}

impl ModelResolver for DefaultModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.aliases.get(model).cloned().unwrap_or_else(|| model.to_string())
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        let resolved = self.resolve(model);
        self.capabilities.get(&resolved)
            .map(|c| c.contains(cap))
            .unwrap_or(false)
    }

    fn max_context(&self, model: &str) -> usize {
        let resolved = self.resolve(model);
        self.context_lengths.get(&resolved).copied().unwrap_or(4096)
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max * 3 / 4, max / 4)
    }
}
```

---

## Phase 3：Pipeline 流水线

### 3.1 Pipeline Stage Trait

**文件**: `src/pipeline/traits.rs`

```rust
use crate::primitive::PrimitiveRequest;

/// 流水线阶段
#[async_trait]
pub trait Stage: Send + Sync {
    /// 阶段名称
    fn name(&self) -> &str;

    /// 处理数据
    async fn process(
        &self,
        context: &mut PipelineContext,
    ) -> crate::Result<()>;
}

/// 流水线上下文
pub struct PipelineContext {
    /// 输入数据（可能是原语或封包数据）
    pub input: PipelineInput,

    /// 输出数据
    pub output: Option<PipelineOutput>,

    /// 当前阶段
    pub current_stage: usize,

    /// 是否直通
    pub passthrough: bool,

    /// URL 构建上下文
    pub url_context: UrlContext,
}

pub enum PipelineInput {
    /// 原语请求
    Primitive(PrimitiveRequest),

    /// 封包数据
    Packed(serde_json::Value),

    /// 原始字节
    Raw(Vec<u8>),
}

pub enum PipelineOutput {
    /// 响应
    Response(crate::provider::LlmResponse),

    /// 流式响应
    Stream(crate::provider::BoxStream<'static, crate::Result<crate::provider::LlmChunk>>),
}
```

### 3.2 各阶段实现

**文件**: `src/pipeline/stages/`

#### 原语化阶段 (primitivize.rs)

```rust
/// 原语化阶段：将输入转换为 PrimitiveRequest
pub struct PrimitivizeStage {
    detector: FormatDetector,
    unwrappers: HashMap<Format, Box<dyn Unwrapper>>,
}

impl Stage for PrimitivizeStage {
    fn name(&self) -> &str { "primitivize" }

    async fn process(&self, ctx: &mut PipelineContext) -> crate::Result<()> {
        match &ctx.input {
            PipelineInput::Primitive(_) => {
                // 已经是原语，跳过
                ctx.passthrough = true;
                Ok(())
            }
            PipelineInput::Packed(data) => {
                // 检测格式并解包
                let format = self.detector.detect(data);
                let unwrapper = self.unwrappers.get(&format).ok_or(...)?;
                let primitive = unwrapper.unwrap(data)?;
                ctx.input = PipelineInput::Primitive(primitive);
                Ok(())
            }
            // ...
        }
    }
}
```

#### 封包阶段 (pack.rs)

```rust
/// 封包阶段：将 PrimitiveRequest 转换为目标格式
pub struct PackStage {
    protocol: Box<dyn ProtocolFormat>,
    hooks: Vec<Box<dyn ProtocolHook>>,
}

impl Stage for PackStage {
    fn name(&self) -> &str { "pack" }

    async fn process(&self, ctx: &mut PipelineContext) -> crate::Result<()> {
        if ctx.passthrough {
            return Ok(());
        }

        if let PipelineInput::Primitive(primitive) = &ctx.input {
            // 从 primitive 获取 stream 标识
            let is_stream = primitive.stream;

            // 封包（传入 is_stream）
            let mut packed = self.protocol.pack(primitive, is_stream);

            // 合并 extra 参数
            for (key, value) in &primitive.extra {
                packed[key] = value.clone();
            }

            // 应用钩子（传入 ctx）
            for hook in &self.hooks {
                hook.after_pack(ctx, &mut packed);
            }

            ctx.input = PipelineInput::Packed(packed);
        }
        Ok(())
    }
}
```

#### 认证阶段 (authenticate.rs)

```rust
/// 认证阶段：注入认证信息
pub struct AuthenticateStage {
    authenticator: Arc<tokio::sync::Mutex<Box<dyn Authenticator>>>,
}

impl Stage for AuthenticateStage {
    fn name(&self) -> &str { "authenticate" }

    async fn process(&self, ctx: &mut PipelineContext) -> crate::Result<()> {
        let mut auth = self.authenticator.lock().await;

        if auth.needs_refresh() {
            auth.refresh().await?;
        }

        // 更新 URL 上下文中的认证类型
        ctx.url_context.auth_type = auth.auth_type();

        Ok(())
    }
}
```

#### 发送阶段 (send.rs)

```rust
/// 发送阶段：发送请求到站点
pub struct SendStage {
    site: Box<dyn Site>,
    authenticator: Arc<tokio::sync::Mutex<Box<dyn Authenticator>>>,
    http: reqwest::Client,
    protocol: Box<dyn ProtocolFormat>,
}

impl Stage for SendStage {
    fn name(&self) -> &str { "send" }

    async fn process(&self, ctx: &mut PipelineContext) -> crate::Result<()> {
        if let PipelineInput::Packed(data) = &ctx.input {
            // 构建 URL（使用上下文）
            let url = self.site.build_url(&ctx.url_context);

            let mut req = self.http.post(&url)
                .json(data)
                .timeout(self.site.timeout());

            // 注入额外 Headers
            for (k, v) in self.site.extra_headers() {
                req = req.header(k, v);
            }

            // 注入认证
            let auth = self.authenticator.lock().await;
            req = auth.inject(req)?;

            // 发送请求
            let resp = req.send().await?;

            // 检查状态码
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let raw = resp.text().await.unwrap_or_default();
                let error = self.protocol.unpack_error(status, &raw)?;
                return Err(crate::Error::Standard(error));
            }

            ctx.output = Some(PipelineOutput::Response(
                self.handle_response(resp).await?
            ));
        }
        Ok(())
    }
}
```

---

## Phase 4：协议实现

### 4.1 OpenAI 协议

**文件**: `src/protocol/base/openai.rs`

```rust
pub struct OpenAIProtocol;

impl ProtocolFormat for OpenAIProtocol {
    fn id(&self) -> &str { "openai" }

    fn pack(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut body = json!({
            "model": primitive.model,
            "messages": [],
        });

        // System message
        let mut messages: Vec<Value> = Vec::new();
        if let Some(system) = &primitive.system {
            messages.push(json!({"role": "system", "content": system}));
        }

        // Messages
        for msg in &primitive.messages {
            messages.push(self.pack_message(msg));
        }
        body["messages"] = json!(messages);

        // Parameters
        self.pack_parameters(&primitive.parameters, &mut body);

        // Tools
        if !primitive.tools.is_empty() {
            body["tools"] = json!(self.pack_tools(&primitive.tools));
        }

        body
    }

    fn unpack_response(&self, raw: &str) -> crate::Result<LlmResponse> {
        // 解析 OpenAI 响应格式
        let json: Value = serde_json::from_str(raw)?;
        // ...
    }

    fn unpack_error(&self, status: u16, raw: &str) -> crate::Result<StandardError> {
        let json: Value = serde_json::from_str(raw)?;
        let error = &json["error"];

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            _ => match error["type"].as_str() {
                Some("context_length_exceeded") => ErrorKind::ContextLengthExceeded,
                Some("content_filter") => ErrorKind::ContentFilter,
                _ => ErrorKind::Other,
            }
        };

        Ok(StandardError {
            kind,
            message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
            code: error["code"].as_str().map(|s| s.to_string()),
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: match kind {
                ErrorKind::RateLimit => Some(FallbackHint::Retry),
                ErrorKind::ModelUnavailable => Some(FallbackHint::FallbackTo("backup".into())),
                _ => Some(FallbackHint::None),
            },
        })
    }

    fn matches_format(&self, data: &Value) -> bool {
        data.get("messages").is_some()
            && data.get("model").is_some()
    }
}
```

### 4.2 Claude 协议

**文件**: `src/protocol/base/claude.rs`

```rust
pub struct ClaudeProtocol;

impl ProtocolFormat for ClaudeProtocol {
    fn id(&self) -> &str { "claude" }

    fn pack(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut body = json!({
            "model": primitive.model,
            "max_tokens": primitive.parameters.max_tokens.unwrap_or(4096),
            "messages": [],
        });

        // System
        if let Some(system) = &primitive.system {
            body["system"] = json!([{"type": "text", "text": system}]);
        }

        // Messages with content blocks
        let messages: Vec<Value> = primitive.messages.iter()
            .map(|m| self.pack_message(m))
            .collect();
        body["messages"] = json!(messages);

        body
    }

    fn unpack_error(&self, status: u16, raw: &str) -> crate::Result<StandardError> {
        let json: Value = serde_json::from_str(raw)?;
        let error = &json["error"];

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            _ => match error["type"].as_str() {
                Some("context_length_exceeded") => ErrorKind::ContextLengthExceeded,
                Some("content_filter") => ErrorKind::ContentFilter,
                _ => ErrorKind::Other,
            }
        };

        Ok(StandardError {
            kind,
            message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
            code: error["type"].as_str().map(|s| s.to_string()),
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: None,
        })
    }

    fn matches_format(&self, data: &Value) -> bool {
        // Claude 特征：system 是数组
        data.get("system")
            .and_then(|s| s.as_array())
            .is_some()
    }
}
```

### 4.3 Gemini 协议

**文件**: `src/protocol/base/gemini.rs`

```rust
pub struct GeminiProtocol;

impl ProtocolFormat for GeminiProtocol {
    fn id(&self) -> &str { "gemini" }

    fn pack(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut body = json!({});

        // System Instruction
        if let Some(system) = &primitive.system {
            body["systemInstruction"] = json!({
                "parts": [{"text": system}]
            });
        }

        // Contents (role: "user" | "model")
        let contents: Vec<Value> = primitive.messages.iter()
            .map(|m| self.pack_message(m))
            .collect();
        body["contents"] = json!(contents);

        // Generation Config
        self.pack_generation_config(&primitive.parameters, &mut body);

        body
    }

    fn unpack_error(&self, status: u16, raw: &str) -> crate::Result<StandardError> {
        let json: Value = serde_json::from_str(raw)?;
        let error = &json["error"];

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            _ => match error["status"].as_str() {
                Some("RESOURCE_EXHAUSTED") => ErrorKind::RateLimit,
                Some("INVALID_ARGUMENT") => ErrorKind::ContextLengthExceeded,
                _ => ErrorKind::Other,
            }
        };

        Ok(StandardError {
            kind,
            message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
            code: error["status"].as_str().map(|s| s.to_string()),
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: None,
        })
    }

    fn matches_format(&self, data: &Value) -> bool {
        data.get("contents").is_some()
            && data.get("parts").is_none() // 区分 CloudCode
    }
}
```

---

## Phase 5：预设平台

### 5.1 注册表

**文件**: `src/presets/registry.rs`

```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;

/// 平台预设注册表
pub struct PresetRegistry {
    presets: HashMap<String, PlatformPreset>,
}

impl PresetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            presets: HashMap::new(),
        };

        // 注册所有预设
        registry.register(openai::preset());
        registry.register(anthropic::preset());
        registry.register(gemini::preset());
        registry.register(vertex::preset());
        registry.register(deepseek::preset());
        registry.register(moonshot::preset());
        registry.register(zhipu::preset());
        registry.register(iflow::preset());
        registry.register(openrouter::preset());
        registry.register(gemini_cli::preset());
        registry.register(antigravity::preset());

        registry
    }

    pub fn register(&mut self, preset: PlatformPreset) {
        self.presets.insert(preset.id.clone(), preset);
    }

    pub fn get(&self, id: &str) -> Option<&PlatformPreset> {
        self.presets.get(id)
    }

    pub fn list(&self) -> Vec<&str> {
        self.presets.keys().map(|s| s.as_str()).collect()
    }
}

/// 全局注册表
pub static REGISTRY: Lazy<PresetRegistry> = Lazy::new(PresetRegistry::new);
```

### 5.2 预设定义示例

**文件**: `src/presets/openai.rs`

```rust
use crate::preset::{PlatformPreset, PresetBuilder};
use crate::protocol::base::OpenAIProtocol;
use crate::site::SimpleSite;
use crate::model::DefaultModelResolver;

pub fn preset() -> PlatformPreset {
    PresetBuilder::new("openai")
        .name("OpenAI Official")
        .site(SimpleSite {
            id: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            extra_headers: HashMap::new(),
            timeout: Duration::from_secs(60),
        })
        .auth(AuthMethod::ApiKey)
        .protocol(OpenAIProtocol)
        .model_resolver(DefaultModelResolver::new())
        .default_models(vec![
            "gpt-4o",
            "gpt-4-turbo",
            "gpt-3.5-turbo",
        ])
        .build()
}
```

**文件**: `src/presets/vertex.rs`

```rust
use crate::protocol::hooks::IFlowThinkingHook;
use crate::site::VertexSite;

pub fn preset() -> PlatformPreset {
    PresetBuilder::new("vertex")
        .name("Vertex AI")
        .site(VertexSite::new())  // 使用自定义 Site，支持认证类型依赖的 URL
        .auth(AuthMethod::ServiceAccount)
        .protocol(GeminiProtocol)
        .model_resolver(VertexModelResolver::new())
        .default_models(vec![
            "gemini-2.5-flash",
            "gemini-2.5-pro",
        ])
        .build()
}
```

**文件**: `src/presets/iflow.rs`

```rust
use crate::protocol::hooks::IFlowThinkingHook;

pub fn preset() -> PlatformPreset {
    PresetBuilder::new("iflow")
        .name("iFlow (心流)")
        .site(SimpleSite {
            id: "iflow".into(),
            base_url: "https://apis.iflow.cn/v1".into(),
            extra_headers: HashMap::new(),
            timeout: Duration::from_secs(60),
        })
        .auth(AuthMethod::Cookie)
        .protocol(OpenAIProtocol)
        .protocol_hook(IFlowThinkingHook)
        .model_resolver(IFlowModelResolver::new())
        .default_models(vec![
            "qwen3-max",
            "deepseek-v3.2",
            "glm-4-plus",
        ])
        .build()
}
```

---

## Phase 6：客户端 API

### 6.1 LlmClient Builder

**文件**: `src/client.rs`

```rust
use crate::model::{ModelResolver, Capability};

/// LLM 客户端
pub struct LlmClient {
    site: Box<dyn Site>,
    authenticator: Arc<tokio::sync::Mutex<Box<dyn Authenticator>>>,
    protocol: Box<dyn ProtocolFormat>,
    protocol_hooks: Vec<Box<dyn ProtocolHook>>,
    model_resolver: Box<dyn ModelResolver>,
    default_model: String,
    http: reqwest::Client,
}

impl LlmClient {
    /// 从预设创建
    pub fn from_preset(id: &str) -> ClientBuilder {
        let preset = REGISTRY.get(id)
            .unwrap_or_else(|| panic!("Unknown preset: {}", id));
        ClientBuilder::from_preset(preset)
    }

    /// 创建 builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// 执行请求
    pub async fn complete(
        &self,
        primitive: PrimitiveRequest,
    ) -> crate::Result<LlmResponse> {
        let mut ctx = PipelineContext::from_primitive(primitive);

        let pipeline = self.build_pipeline(false);
        pipeline.execute(&mut ctx).await?;

        ctx.take_response()
    }

    /// 流式执行
    pub async fn stream(
        &self,
        primitive: PrimitiveRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<LlmChunk>>> {
        let mut ctx = PipelineContext::from_primitive(primitive);

        let pipeline = self.build_pipeline(true);
        pipeline.execute(&mut ctx).await?;

        ctx.take_stream()
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
}

/// 客户端构建器
pub struct ClientBuilder {
    site: Option<Box<dyn Site>>,
    authenticator: Option<Box<dyn Authenticator>>,
    protocol: Option<Box<dyn ProtocolFormat>>,
    protocol_hooks: Vec<Box<dyn ProtocolHook>>,
    model_resolver: Option<Box<dyn ModelResolver>>,
    default_model: Option<String>,
    http: Option<reqwest::Client>,
}

impl ClientBuilder {
    pub fn site(mut self, url: impl Into<String>) -> Self {
        self.site = Some(Box::new(SimpleSite {
            id: "custom".into(),
            base_url: url.into(),
            extra_headers: HashMap::new(),
            timeout: Duration::from_secs(60),
        }));
        self
    }

    pub fn auth(mut self, auth: impl Into<Box<dyn Authenticator>>) -> Self {
        self.authenticator = Some(auth.into());
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.authenticator = Some(Box::new(ApiKeyAuth::new(key.into())));
        self
    }

    pub fn protocol(mut self, protocol: impl Into<Box<dyn ProtocolFormat>>) -> Self {
        self.protocol = Some(protocol.into());
        self
    }

    pub fn protocol_hook(mut self, hook: impl Into<Box<dyn ProtocolHook>>) -> Self {
        self.protocol_hooks.push(hook.into());
        self
    }

    pub fn model_resolver(mut self, resolver: impl Into<Box<dyn ModelResolver>>) -> Self {
        self.model_resolver = Some(resolver.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    pub fn build(self) -> LlmClient {
        LlmClient {
            site: self.site.expect("site is required"),
            authenticator: Arc::new(tokio::sync::Mutex::new(
                self.authenticator.expect("auth is required")
            )),
            protocol: self.protocol.expect("protocol is required"),
            protocol_hooks: self.protocol_hooks,
            model_resolver: self.model_resolver
                .unwrap_or_else(|| Box::new(DefaultModelResolver::new())),
            default_model: self.default_model.unwrap_or_default(),
            http: self.http.unwrap_or_default(),
        }
    }
}
```

---

## Phase 7：迁移策略

### 7.1 保留旧 API 兼容

```rust
// 旧 API（保留，内部使用新架构）
pub type GeminiProvider = LlmClient;
pub type IFlowProvider = LlmClient;

impl GeminiProvider {
    pub fn new(config: GeminiConfig, http: reqwest::Client) -> Self {
        LlmClient::builder()
            .site(config.base_url())
            .api_key(config.api_key)
            .protocol(Protocol::gemini())
            .model_resolver(GeminiModelResolver::new())
            .build()
    }
}
```

### 7.2 渐进迁移

1. 先实现新架构
2. 旧 Provider 内部迁移到新架构
3. 保留旧 API 作为便捷方法
4. 文档推荐使用新 API

---

## 实施时间表

| Phase | 内容 | 预计时间 |
|-------|------|----------|
| 1 | 核心 Traits 定义（含错误规范化） | 1.5 天 |
| 2 | 模型解析器 | 0.5 天 |
| 3 | Pipeline 流水线 | 2 天 |
| 4 | 协议实现（含错误解包） | 2 天 |
| 5 | 预设平台 | 1 天 |
| 6 | 客户端 API | 1 天 |
| 7 | 迁移和测试 | 2 天 |

**总计：约 10 天**

---

## 变更日志

### v2.2 新增内��

1. **ProtocolHook 生命周期扩展**：
   - 钩子方法签名改为接收 `&mut PipelineContext`
   - 新增 `before_send` 钩子（可修改 headers）
   - 新增 `after_receive` 钩子（流式响应预处理）

2. **流式请求处理**：
   - `PrimitiveRequest` 新增 `stream: bool` 字段
   - `ProtocolFormat::pack()` 新增 `is_stream` 参数
   - 支持 URL 层（`Action::Stream`）和 JSON Body 层（`"stream": true`）双轨处理

### v2.1 新增内容

1. **错误规范化**：新增 `StandardError`、`ErrorKind`、`FallbackHint`，`ProtocolFormat` 添加 `unpack_error` 方法
2. **URL 构建上下文**：新增 `UrlContext`、`Action`、`AuthType`，`Site::build_url` 接收上下文参数
3. **模型解析器**：新增 `ModelResolver` trait 和 `Capability` 标志，支持别名解析和能力检测
4. **平台特定参数**：`PrimitiveRequest` 添加 `extra` 字段，封包阶段合并到请求体
