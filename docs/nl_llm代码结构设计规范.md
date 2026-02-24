# nl_llm 代码结构设计规范 v2.2

## 概述

本文档定义了 `nl_llm` 模块的完整架构设计。采用**四维正交分解**架构，实现站点、协议、认证、模型的自由组合。

### 核心原则

1. **四维正交分解**：Site（站点）、Protocol（协议）、Auth（认证）、Model（模型）四个维度完全独立
2. **预设 + 组装双层 API**：提供开箱即用的预设平台，同时支持灵活自定义组装
3. **原语中间层**：Primitive 作为统一中间表示，解耦输入解析和输出生成
4. **流水线处理**：数据原语化 → 封包 → 认证 → 发送 → 解包
5. **错误规范化**：平台错误统一转换为标准错误，携带重试/降级信号

---

## 1. 架构总览

### 1.1 四维正交分解

```
┌─────────────────────────────────────────────────────────────────┐
│                        LlmClient                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Pipeline（流水线）                        ││
│  │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐     ││
│  │  │ 原语化  │ → │  封包   │ → │  认证   │ → │  发送   │     ││
│  │  └─────────┘   └─────────┘   └─────────┘   └─────────┘     ││
│  │       ↓             ↓             ↓             ↓          ││
│  │  [Primitive]   [Protocol]     [Auth]       [Site]          ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 维度定义

| 维度 | 职责 | 示例 |
|------|------|------|
| **Site** | 站点：Base URL、HTTP 配置、超时设置 | OpenAI 官方、iFlow、自定义代理 |
| **Auth** | 认证：如何获取和注入凭证 | API Key、OAuth、Service Account、Cookie |
| **Protocol** | 协议：如何封包/解包数据 | OpenAI 格式、Claude 格式、Gemini 格式 |
| **Model** | 模型：模型标识和能力 | gpt-4o、claude-3-opus、gemini-2.5-pro |

### 1.3 组装示例

```rust
// 方式1：使用预设（简单）
let client = LlmClient::from_preset("iflow")
    .with_auth("BXAuth=xxx")
    .build();

// 方式2：自定义组装（灵活）
let client = LlmClient::builder()
    .site("https://api.example.com/v1")
    .auth(Auth::api_key("sk-xxx"))
    .protocol(Protocol::openai())
    .model("gpt-4o")
    .build();

// 方式3：基于预设修改
let client = LlmClient::from_preset("openai")
    .with_base_url("https://proxy.example.com/v1")
    .build();
```

---

## 2. 数据处理流水线

### 2.1 流水线阶段

```
输入数据
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 阶段 1: 数据原语化 (Primitivize)                             │
│   - 如果输入已是 PrimitiveRequest → 跳过                     │
│   - 如果输入是封包数据 → 解包为 PrimitiveRequest              │
│   - 如果格式与目标相同 → 可直通                              │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
PrimitiveRequest（原语）
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 阶段 2: 数据封包 (Pack)                                      │
│   - 根据 Protocol 格式封包                                   │
│   - 应用协议变体钩子（如 iFlow Thinking 字段）               │
│   - 如果格式与输入相同 → 可直通                              │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
封包数据（JSON）
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 阶段 3: 认证注入 (Authenticate)                              │
│   - 如果需要 Token 刷新 → 执行刷新                           │
│   - 注入认证 Header / URL 参数                               │
│   - 如果是代理站 API Key → 直接使用                          │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
认证后的请求
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 阶段 4: 数据发送 (Send)                                      │
│   - 发送到 Site 指定的端点                                   │
│   - 处理网络错误、重试                                       │
│   - 错误解包与规范化                                         │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ 阶段 5: 数据解包 (Unpack)                                    │
│   - 根据 Protocol 格式解包响应                               │
│   - 解析为 LlmResponse / LlmChunk                           │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
输出数据
```

### 2.2 直通优化

当输入格式与目标格式相同时，跳过解包和封包步骤：

```
输入: OpenAI 格式
目标: OpenAI 格式
结果: 直通（不解包不封包）
```

### 2.3 流式请求处理

流式请求的处理涉及两个层面：

| 处理层 | 方式 | 示例平台 |
|--------|------|----------|
| **URL 层** | `Site::build_url` 根据 `Action::Stream` 添加参数 | Vertex AI: `?alt=sse` |
| **JSON Body 层** | `Protocol::pack(primitive, true)` 添加 `stream` 字段 | OpenAI/Claude: `"stream": true` |

两种方式可以同时生效，例如某平台既在 URL 区分端点，又需要在 body 中声明流式。

**数据流**：
```
PrimitiveRequest { stream: true }
    │
    ├─→ Site::build_url(ctx) where ctx.action = Action::Stream
    │       → URL: .../streamGenerateContent?alt=sse
    │
    └─→ Protocol::pack(primitive, is_stream=true)
            → JSON: { ..., "stream": true }
```

---

## 3. 协议格式

### 3.1 核心协议（3种）

| 协议 | 特征 | JSON 结构概要 |
|------|------|--------------|
| **OpenAI** | `messages` 数组、`tools`、`function` | `{"messages": [...], "model": "..."}` |
| **Claude** | `system` 数组、`content` 数组、`tool_use` | `{"messages": [...], "system": [...]}` |
| **Gemini** | `contents` 数组、`role: "model"`、`parts` | `{"contents": [...], "systemInstruction": {...}}` |

### 3.2 协议变体

部分平台对基础协议有特殊扩展：

| 变体 | 基于协议 | 扩展内容 |
|------|----------|----------|
| **CloudCode** | Gemini | 添加 `requestType: "agent"`、`project`、`sessionId` |
| **iFlow OpenAI** | OpenAI | 添加 `chat_template_kwargs.enable_thinking` |
| **OpenRouter** | OpenAI | 添加 `provider` 字段 |

#### CloudCode + Claude 模型路由

Antigravity / CloudCode PA 平台除了支持 Gemini 模型外，还通过翻译层支持 Claude 模型。
Claude 请求经由 CLIProxyAPI 的 `ConvertClaudeRequestToAntigravity` 翻译器转换为 Gemini 格式后发送到 CloudCode PA 端点。

**关键特性**：
- Claude 模型名称直接使用（如 `claude-opus-4-6-thinking`、`claude-sonnet-4-6`）
- 模型名中含 `"claude"` 的请求会走 Claude 翻译分支
- **不能通过 `generateContent` 端点直接探测 Claude 模型**（会返回 404）

**CloudCode PA 内部模型名映射**（参照 `defaultAntigravityAliases()`）：

| 对外名称 | 内部名称 |
|----------|----------|
| `gemini-3-pro-preview` | `gemini-3-pro-high` |
| `gemini-3.1-pro-preview` | `gemini-3.1-pro-high` |
| `gemini-3-flash-preview` | `gemini-3-flash` |
| `gemini-3-pro-image-preview` | `gemini-3-pro-image` |

### 3.6 CloudCode PA 模型发现（特例）

> [!NOTE]
> `fetchAvailableModels` 是 CloudCode PA 独有的端点，并非通用协议的一部分。
> 仅在需要动态发现 Antigravity / Gemini CLI 平台可用模型列表时使用。

CloudCode PA 提供 `POST /v1internal:fetchAvailableModels`（body: `{}`）端点，
一次性返回所有可用模型（包括 Gemini 和 Claude），比逐个探测更高效可靠。

### 3.3 协议变体处理方式

采用**后处理钩子**模式，钩子可以访问和修改 PipelineContext：

```rust
use crate::pipeline::PipelineContext;

/// 协议钩子（扩展签名）
pub trait ProtocolHook: Send + Sync {
    /// 封包后处理
    fn after_pack(&self, ctx: &mut PipelineContext, packed: &mut Value) {
        // 默认空实现
    }

    /// 解包前处理
    fn before_unpack(&self, ctx: &mut PipelineContext, data: &mut Value) {
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

// 示例：iFlow Thinking 钩子
struct IFlowThinkingHook;
impl ProtocolHook for IFlowThinkingHook {
    fn after_pack(&self, ctx: &mut PipelineContext, packed: &mut Value) {
        if let PipelineInput::Primitive(primitive) = &ctx.input {
            if is_thinking_model(&primitive.model) {
                packed["chat_template_kwargs"] = json!({"enable_thinking": true});
                packed["reasoning_split"] = json!(true);
            }
        }
    }
}

// 示例：需要添加额外 Headers 的钩子
struct CustomHeaderHook;
impl ProtocolHook for CustomHeaderHook {
    fn before_send(&self, _ctx: &mut PipelineContext, req: &mut reqwest::RequestBuilder) {
        *req = req.header("X-Custom-Header", "value");
    }
}
```

### 3.4 协议格式 Trait（完整定义）

```rust
use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, LlmChunk};

/// 协议格式定义
pub trait ProtocolFormat: Send + Sync {
    /// 协议标识
    fn id(&self) -> &str;

    /// 封包：PrimitiveRequest → JSON
    /// is_stream 参数用于在 JSON body 中添加 stream 标识（如 OpenAI 的 "stream": true）
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
    /// 此方法在 HTTP 响应非 2xx 时调用
    fn unpack_error(&self, status: u16, raw: &str) -> crate::Result<StandardError>;
}
```

**说明**：`pack()` 方法接收 `is_stream` 参数，因为：
- Vertex AI: 通过 URL 区分（`?alt=sse`），由 `Site::build_url` 处理
- OpenAI/Claude: 通过 JSON body 区分（`"stream": true`），由 `Protocol::pack` 处理
- 两种方式不冲突，可以同时使用

### 3.5 错误规范化

将各平台特有的错误格式统一转换为 `StandardError`：

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

// 示例：OpenAI 错误解包
impl ProtocolFormat for OpenAIProtocol {
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
}
```

---

## 4. 认证方式

### 4.1 认证类型

| 类型 | 特点 | 使用场景 |
|------|------|---------|
| **API Key** | 直接使用、无过期 | 大多数平台 |
| **OAuth** | 需登录、Token 过期需刷新 | Claude OAuth、Gemini CLI |
| **Service Account** | JWT 签名、GCP 专用 | Vertex AI |
| **Cookie** | 网页 Cookie 换取 Token | iFlow |
| **MultiKey** | 多字段动态签名 | 讯飞星火、百度文心 |

### 4.2 认证接口

```rust
trait Authenticator: Send + Sync {
    /// 是否已认证
    fn is_authenticated(&self) -> bool;

    /// 是否需要刷新
    fn needs_refresh(&self) -> bool;

    /// 刷新认证
    async fn refresh(&mut self) -> Result<()>;

    /// 注入认证信息到请求
    fn inject(&self, req: RequestBuilder) -> Result<RequestBuilder>;
}
```

### 4.3 动态 OAuth 配置（DynamicOAuthConfig）

对于 Antigravity 等 OAuth 认证平台，支持运行时自定义 OAuth 配置（如使用自己的 OAuth 应用）：

```rust
/// 动态 OAuth 配置（Owned 版本，用于运行时构造）
#[derive(Clone)]
pub struct DynamicOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_port: u16,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

// 使用示例
let custom_config = DynamicOAuthConfig {
    client_id: "my-oauth-app-id".into(),
    ..DynamicOAuthConfig::default()  // 其余使用默认值
};

let auth = AntigravityOAuth::new()
    .with_config(custom_config)
    .with_verbose(true)  // 启用调试日志
    .with_cache("path/to/cache");
```

---

## 5. 站点定义

### 5.1 站点属性

```rust
struct Site {
    /// 站点标识
    id: String,
    /// Base URL
    base_url: String,
    /// 额外 Headers（借用形式，生命周期绑定 &self）
    extra_headers: HashMap<&str, &str>,
    /// 超时设置
    timeout: Duration,
}
```

### 5.2 站点类型

站点只是配置，不区分类型：
- 可以是官方端点
- 可以是代理站
- 可以是本地服务

### 5.3 URL 构建上下文（新增）

部分平台的 URL 结构依赖认证类型和操作类型，因此 `build_url` 方法需要接收上下文：

```rust
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

/// 操作类型枚举
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

/// 认证类型枚举（用于 URL 构建）
pub enum AuthType {
    ApiKey,
    OAuth,
    ServiceAccount,
    Cookie,
    MultiKey,
}

/// Site Trait（完整定义）
pub trait Site: Send + Sync {
    /// 站点标识
    fn id(&self) -> &str;

    /// 获取 Base URL
    fn base_url(&self) -> &str;

    /// 构建完整请求 URL
    /// ctx 包含模型、认证类型、操作类型等信息
    fn build_url(&self, ctx: &UrlContext) -> String;

    /// 获取额外 Headers
    /// 返回 HashMap<&str, &str>，值可以借用 &self 字段
    fn extra_headers(&self) -> HashMap<&str, &str>;

    /// 获取超时设置
    fn timeout(&self) -> Duration;
}

// 示例：Vertex AI 的 URL 构建
impl Site for VertexSite {
    fn build_url(&self, ctx: &UrlContext) -> String {
        let action_suffix = match ctx.action {
            Action::Generate => "generateContent",
            Action::Stream => "streamGenerateContent?alt=sse",
            _ => "generateContent",
        };

        // 根据 auth_type 选择不同的 URL 结构
        let base = match ctx.auth_type {
            AuthType::ServiceAccount => {
                // SA 认证：使用标准 Vertex AI 端点
                format!(
                    "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:{}",
                    self.location, self.project_id, self.location, ctx.model, action_suffix
                )
            }
            AuthType::ApiKey => {
                // API Key 认证：使用简化端点
                format!(
                    "https://us-central1-aiplatform.googleapis.com/v1beta1/projects/{}/locations/us-central1/publishers/google/models/{}:{}",
                    self.project_id, ctx.model, action_suffix
                )
            }
            _ => self.base_url.clone(),
        };

        base
    }
}
```

---

## 6. 模型解析（新增）

### 6.1 模型解析器 Trait

用于处理模型别名和能力检测：

```rust
/// 模型能力标志
bitflags::bitflags! {
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
    /// 例如: "gpt4" → "gpt-4o", "claude" → "claude-sonnet-4-20250514"
    fn resolve(&self, model: &str) -> String;

    /// 检查模型是否支持指定能力
    fn has_capability(&self, model: &str, cap: Capability) -> bool;

    /// 获取模型的最大上下文长度
    fn max_context(&self, model: &str) -> usize;

    /// 获取模型的上下文窗口建议（输入/输出分配）
    fn context_window_hint(&self, model: &str) -> (usize, usize);
}
```

### 6.2 默认模型解析器实现

```rust
/// 默认模型解析器
pub struct DefaultModelResolver {
    /// 模型别名映射
    aliases: HashMap<String, String>,
    /// 模型能力表
    capabilities: HashMap<String, Capability>,
    /// 模型上下文长度表
    context_lengths: HashMap<String, usize>,
}

impl DefaultModelResolver {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        // OpenAI 别名
        aliases.insert("gpt4".into(), "gpt-4o".into());
        aliases.insert("gpt4-turbo".into(), "gpt-4-turbo".into());
        aliases.insert("gpt3".into(), "gpt-3.5-turbo".into());
        // Claude 别名 (Official API)
        aliases.insert("claude".into(), "claude-sonnet-4-20250514".into());
        aliases.insert("claude-opus".into(), "claude-opus-4-20250514".into());
        aliases.insert("claude-sonnet".into(), "claude-sonnet-4-20250514".into());
        // Gemini 别名
        aliases.insert("gemini".into(), "gemini-2.5-flash".into());
        aliases.insert("gemini-pro".into(), "gemini-2.5-pro".into());
        aliases.insert("gemini-flash".into(), "gemini-2.5-flash".into());
        // 注意：Antigravity 专属别名由 AntigravityModelResolver 提供

        let mut capabilities = HashMap::new();
        // OpenAI
        capabilities.insert("gpt-4o".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        capabilities.insert("gpt-4-turbo".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        // Claude
        capabilities.insert("claude-sonnet-4-20250514".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        capabilities.insert("claude-opus-4-20250514".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        // Gemini
        capabilities.insert("gemini-2.5-flash".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING);
        capabilities.insert("gemini-2.5-pro".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING | Capability::CODE_EXECUTION);

        let mut context_lengths = HashMap::new();
        context_lengths.insert("gpt-4o".into(), 128_000);
        context_lengths.insert("gpt-4-turbo".into(), 128_000);
        context_lengths.insert("claude-sonnet-4-20250514".into(), 200_000);
        context_lengths.insert("claude-opus-4-20250514".into(), 200_000);
        context_lengths.insert("gemini-2.5-flash".into(), 1_000_000);
        context_lengths.insert("gemini-2.5-pro".into(), 1_000_000);

        Self { aliases, capabilities, context_lengths }
    }

    /// 设置或覆盖模型别名（供预设级 Resolver 扩展）
    pub fn set_alias(&mut self, alias: impl Into<String>, model: impl Into<String>) { ... }
    pub fn set_capability(&mut self, model: impl Into<String>, cap: Capability) { ... }
    pub fn set_context_length(&mut self, model: impl Into<String>, length: usize) { ... }
    pub fn extend_aliases(&mut self, aliases: Vec<(impl Into<String>, impl Into<String>)>) { ... }
    pub fn extend_capabilities(&mut self, caps: Vec<(impl Into<String>, Capability)>) { ... }
    pub fn extend_context_lengths(&mut self, lengths: Vec<(impl Into<String>, usize)>) { ... }
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
        // 默认保留 1/4 作为输出
        (max * 3 / 4, max / 4)
    }
}
```

### 6.3 预设级模型解析器

> [!NOTE]
> 不同预设可能需要不同的模型别名映射。例如 `claude-opus` 在官方 Claude API 和 Antigravity 平台解析为不同的模型名。

预设级 ModelResolver 基于 `DefaultModelResolver` 扩展，覆盖特定平台的别名和能力：

```rust
/// Antigravity / CloudCode PA 专属模型解析器
pub struct AntigravityModelResolver {
    inner: DefaultModelResolver,
}

impl AntigravityModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // 覆盖 Claude 别名 → Antigravity 版本
        inner.set_alias("claude-opus", "claude-opus-4-6-thinking");
        inner.set_alias("claude-sonnet", "claude-sonnet-4-6");

        // Gemini 3.x 预览版 → CloudCode PA 内部名称
        inner.extend_aliases(vec![
            ("gemini-3-pro-preview", "gemini-3-pro-high"),
            ("gemini-3.1-pro-preview", "gemini-3.1-pro-high"),
            ("gemini-3-flash-preview", "gemini-3-flash"),
            ("gemini-3-pro-image-preview", "gemini-3-pro-image"),
        ]);

        // 添加 Antigravity 特有的模型能力和上下文长度配置...
        Self { inner }
    }
}
```

预设配置中使用：

```rust
// crates/nl_llm_v2/src/presets/antigravity.rs
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(CloudCodeSite::new())
        .protocol(GeminiProtocol {})
        .protocol_hook(CloudCodeHook {})
        .model_resolver(AntigravityModelResolver::new())  // 使用专属 Resolver
        .default_model("gemini-2.5-flash")
}
```

### 6.4 iFlow 平台模型解析器示例

```rust
/// iFlow 平台专属模型解析器
pub struct IFlowModelResolver {
    inner: DefaultModelResolver,
}

impl IFlowModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // 添加 iFlow 平台特有的模型别名
        inner.extend_aliases(vec![
            ("qwen", "qwen3-max"),
            ("qwen-max", "qwen3-max"),
            ("glm", "glm-4-flash"),
            ("deepseek", "deepseek-v3"),
        ]);

        // 配置 iFlow 平台模型能力
        inner.extend_capabilities(vec![
            ("qwen3-max", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("glm-4", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("deepseek-r1", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // 配置上下文长度...
        Self { inner }
    }
}

// 预设配置中使用
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(IFlowSite::new())
        .protocol(OpenAiProtocol {})
        .protocol_hook(IflowThinkingHook {})
        .model_resolver(IFlowModelResolver::new())
        .default_model("qwen3-max")
}
```

---

## 7. 原语格式（完整定义）

### 7.1 PrimitiveRequest

```rust
/// 原语请求：统一中间表示
pub struct PrimitiveRequest {
    /// 模型名称（可使用别名，由 ModelResolver 解析）
    pub model: String,

    /// 系统提示
    pub system: Option<String>,

    /// 消息列表
    pub messages: Vec<PrimitiveMessage>,

    /// 工具定义
    pub tools: Vec<PrimitiveTool>,

    /// 生成参数
    pub parameters: PrimitiveParameters,

    /// 元数据
    pub metadata: PrimitiveMetadata,

    /// 是否流式请求
    /// 用于：
    /// 1. Site::build_url 根据 Action::Stream 构建 URL（如 Vertex AI 的 ?alt=sse）
    /// 2. Protocol::pack 在 JSON body 中添加 stream 标识（如 OpenAI 的 "stream": true）
    pub stream: bool,

    /// 平台特定参数
    /// 用于传递平台特有的配置，如 OpenRouter 的 provider 字段
    /// 这些参数会在封包时合并到请求体中
    pub extra: HashMap<String, serde_json::Value>,
}

/// 原语消息
pub struct PrimitiveMessage {
    pub role: Role,
    pub content: Vec<PrimitiveContent>,
}

/// 原语内容块
pub enum PrimitiveContent {
    Text { text: String },
    Image { url: String, mime_type: Option<String> },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
}

/// 生成参数
pub struct PrimitiveParameters {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Vec<String>,
}

/// 原语工具定义
pub struct PrimitiveTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}
```

### 7.2 平台特定参数使用示例

```rust
// OpenRouter: 指定 provider 偏好
let mut req = PrimitiveRequest::single_user_message("Hello");
req.extra.insert("provider".into(), json!({
    "google": { "only": ["gemini-2.5-pro"] }
}));

// iFlow: 启用思考模式
let mut req = PrimitiveRequest::single_user_message("Solve this problem");
req.extra.insert("chat_template_kwargs".into(), json!({
    "enable_thinking": true
}));

// Vertex AI: 指定区域
let mut req = PrimitiveRequest::single_user_message("Hello");
req.extra.insert("location".into(), json!("asia-northeast1"));
```

---

## 8. 预设平台

### 8.1 预设定义

```rust
struct PlatformPreset {
    /// 平台标识
    id: String,
    /// 平台名称
    name: String,
    /// 站点配置
    site: Site,
    /// 默认认证方式
    default_auth: AuthMethod,
    /// 协议格式
    protocol: ProtocolFormat,
    /// 协议钩子
    protocol_hooks: Vec<Box<dyn ProtocolHook>>,
    /// 模型解析器
    model_resolver: Box<dyn ModelResolver>,
    /// 默认模型列表
    default_models: Vec<String>,
}
```

### 8.2 已支持的平台

| 平台 | 协议 | 认证方式 | 特殊处理 |
|------|------|---------|---------|
| OpenAI | OpenAI | API Key | 无 |
| Anthropic | Claude | API Key (x-api-key) | 专用 AnthropicSite + anthropic-version header |
| Gemini | Gemini | API Key | 无 |
| Vertex AI | Gemini | Service Account / API Key | URL 结构依赖认证类型 |
| DeepSeek | OpenAI | API Key | 无 |
| Moonshot (Kimi) | OpenAI | API Key | 无 |
| 智谱 (GLM) | OpenAI | API Key | 无 |
| iFlow | OpenAI | Cookie | Thinking 钩子 |
| OpenRouter | OpenAI | API Key | Provider 字段 |
| Gemini CLI | CloudCode | OAuth | CloudCode 包装 |
| Antigravity | CloudCode | OAuth | CloudCode 包装，支持 Claude 模型路由 |

---

## 9. 目录结构

```
nl_llm/src/
├── lib.rs                    # 模块入口
│
├── client.rs                 # LlmClient 入口
│
├── presets/                  # 预设平台（平铺，不分类）
│   ├── mod.rs
│   ├── registry.rs          # 平台注册表
│   ├── openai.rs
│   ├── anthropic.rs
│   ├── gemini.rs
│   ├── vertex.rs
│   ├── deepseek.rs
│   ├── moonshot.rs
│   ├── zhipu.rs
│   ├── iflow.rs
│   ├── openrouter.rs
│   ├── gemini_cli.rs
│   ├── antigravity.rs
│   └── ...
│
├── protocol/                 # 协议格式
│   ├── mod.rs
│   ├── traits.rs            # ProtocolFormat trait
│   ├── error.rs             # 【新增】StandardError 定义
│   ├── base/                # 基础协议
│   │   ├── mod.rs
│   │   ├── openai.rs
│   │   ├── claude.rs
│   │   └── gemini.rs
│   └── hooks/               # 协议钩子
│       ├── mod.rs
│       ├── traits.rs
│       ├── iflow.rs
│       └── cloudcode.rs
│
├── site/                     # 站点定义
│   ├── mod.rs
│   ├── traits.rs            # Site trait
│   ├── context.rs           # 【新增】UrlContext, Action 定义
│   └── builder.rs           # Site 构建器
│
├── auth/                     # 认证方式
│   ├── mod.rs
│   ├── types.rs             # 认证类型定义
│   ├── traits.rs            # Authenticator trait
│   └── providers/           # 各平台认证实现
│       ├── mod.rs
│       ├── api_key.rs
│       ├── oauth/
│       ├── service_account.rs
│       ├── cookie.rs
│       └── multikey/
│
├── primitive/                # 原语格式
│   ├── mod.rs
│   ├── request.rs
│   ├── message.rs
│   ├── tool.rs
│   ├── parameters.rs
│   └── metadata.rs
│
├── model/                    # 【新增】模型解析
│   ├── mod.rs
│   ├── resolver.rs          # ModelResolver trait
│   ├── capabilities.rs      # Capability flags
│   └── default.rs           # DefaultModelResolver
│
├── pipeline/                 # 流水线
│   ├── mod.rs
│   ├── traits.rs            # Stage trait
│   ├── stages/
│   │   ├── mod.rs
│   │   ├── primitivize.rs   # 原语化阶段
│   │   ├── pack.rs          # 封包阶段
│   │   ├── authenticate.rs  # 认证阶段
│   │   ├── send.rs          # 发送阶段
│   │   └── unpack.rs        # 解包阶段
│   └── pipeline.rs          # 流水线组装
│
├── gateway.rs               # Gateway 编排层
├── fallback.rs              # 降级路由
└── token_bucket.rs          # 令牌桶限流
```

### 9.2 预设平台示例目录

每个预设平台需要有对应的 `examples/` 子目录，用于测试验证该平台是否可正常交互：

```
examples/                    # 平台示例（按平台分组）
├── openai/
│   ├── chat/               # 基础对话示例
│   │   ├── main.rs
│   │   └── test.bat        # 测试脚本
│   ├── stream/             # 流式输出示例
│   │   ├── main.rs
│   │   └── test.bat
│   └── tools/              # 工具调用示例
│       ├── main.rs
│       └── test.bat
│
├── anthropic/
│   ├── chat/
│   │   ├── main.rs
│   │   └── test.bat
│   └── stream/
│       ├── main.rs
│       └── test.bat
│
├── gemini/
│   ├── chat/
│   └── stream/
│
├── vertex/
│   ├── auth/               # 认证测试（SA / API Key）
│   │   ├── main.rs
│   │   └── test.bat
│   ├── chat/               # 基础对话
│   └── models/             # 模型列表查询
│
├── iflow/
│   ├── auth/               # Cookie 认证测试
│   ├── chat/
│   └── thinking/           # Thinking 模式测试
│
├── gemini_cli/
│   ├── auth/               # OAuth 认证测试
│   └── chat/
│
├── antigravity/
│   ├── auth/               # OAuth 认证测试
│   ├── chat/               # 基础对话
│   ├── stream/             # 流式输出
│   └── models/             # 模型列表查询（fetchAvailableModels）
│
├── deepseek/
│   └── chat/
│
├── moonshot/
│   └── chat/
│
├── zhipu/
│   └── chat/
│
└── openrouter/
    └── chat/
```

**示例目录命名规范**：
- `chat/` - 基础对话测试
- `stream/` - 流式输出测试
- `tools/` - 工具调用测试
- `auth/` - 认证流程测试
- `models/` - 模型列表查询
- `thinking/` - 思考模式测试（特定平台）
- `vision/` - 视觉能力测试
- `embed/` - 向量嵌入测试

**每个示例目录必须包含**：
- `main.rs` - Rust 示例代码
- `test.bat` - Windows 批处理测试脚本（便于快速测试单个案例）

### 9.3 自动测试要求

**重要**：每次修改核心模块时，必须运行受影响预设平台的测试脚本：

| 修改模块 | 需测试的平台 |
|----------|--------------|
| `protocol/base/openai.rs` | openai, deepseek, moonshot, zhipu, iflow, openrouter |
| `protocol/base/claude.rs` | anthropic |
| `protocol/base/gemini.rs` | gemini, vertex, gemini_cli, antigravity |
| `protocol/hooks/iflow.rs` | iflow |
| `protocol/hooks/cloudcode.rs` | gemini_cli, antigravity |
| `auth/providers/api_key.rs` | 所有使用 API Key 的平台 |
| `auth/providers/oauth/` | anthropic, gemini_cli, antigravity |
| `auth/providers/service_account.rs` | vertex |
| `auth/providers/cookie.rs` | iflow |
| `site/context.rs` | 所有平台 |
| `pipeline/` | 所有平台 |

**测试原则**：
1. 修改代码后，识别受影响的平台
2. 运行对应平台的 `test.bat` 脚本
3. 确认所有测试通过后再提交

---

## 10. 使用示例

### 10.1 使用预设平台

```rust
// OpenAI
let client = LlmClient::from_preset("openai")
    .with_api_key("sk-xxx")
    .build();

// iFlow
let client = LlmClient::from_preset("iflow")
    .with_cookie("BXAuth=xxx")
    .build();

// Vertex AI (Service Account)
let client = LlmClient::from_preset("vertex")
    .with_service_account_json(json_str)
    .build();

// Vertex AI (API Key)
let client = LlmClient::from_preset("vertex")
    .with_api_key("AIza...")
    .build();

// Anthropic (使用 x-api-key，非标准 Bearer)
let client = LlmClient::from_preset("anthropic")
    .with_anthropic_api_key("sk-ant-xxx")
    .build();
```

### 10.2 自定义组装

```rust
// 代理站 + OpenAI 格式
let client = LlmClient::builder()
    .site("https://proxy.example.com/v1")
    .auth(Auth::api_key("sk-xxx"))
    .protocol(Protocol::openai())
    .model("gpt-4o")
    .build();

// 代理站 + Claude 格式
let client = LlmClient::builder()
    .site("https://proxy.example.com/v1")
    .auth(Auth::api_key("sk-xxx"))
    .protocol(Protocol::claude())
    .model("claude-3-opus")
    .build();

// 自定义协议钩子
let client = LlmClient::builder()
    .site("https://custom.com/v1")
    .auth(Auth::api_key("sk-xxx"))
    .protocol(Protocol::openai())
    .protocol_hook(CustomThinkingHook)
    .build();
```

### 10.3 基于预设修改

```rust
// 使用 OpenAI 预设，但修改端点
let client = LlmClient::from_preset("openai")
    .with_base_url("https://api.custom-proxy.com/v1")
    .build();

// 使用 iFlow 预设，但使用不同的模型
let client = LlmClient::from_preset("iflow")
    .with_cookie("BXAuth=xxx")
    .with_model("claude-3-opus")
    .build();
```

### 10.4 使用模型别名和能力检测

```rust
let client = LlmClient::from_preset("openai")
    .with_api_key("sk-xxx")
    .build();

// 使用别名
let req = PrimitiveRequest::single_user_message("Hello")
    .with_model("gpt4");  // 会解析为 "gpt-4o"

// 检查能力
if client.has_capability("gpt4", Capability::VISION) {
    // 支持 Vision
}

// 获取上下文窗口建议
let (input_limit, output_limit) = client.context_window_hint("gpt4");
```

### 10.5 使用平台特定参数

```rust
// OpenRouter: 指定 provider
let mut req = PrimitiveRequest::single_user_message("Hello")
    .with_model("gemini-2.5-pro");
req.extra.insert("provider".into(), json!({
    "google": { "only": ["gemini-2.5-pro"] }
}));

let response = client.complete(req).await?;
```

---

## 11. 设计原则总结

| 序号 | 原则 | 说明 |
|------|------|------|
| 1 | **四维正交** | Site、Protocol、Auth、Model 四个维度完全独立 |
| 2 | **预设优先** | 提供开箱即用的预设平台，降低使用门槛 |
| 3 | **组装灵活** | 支持自定义组装，满足定制需求 |
| 4 | **协议复用** | 协议格式与平台无关，可在不同平台复用 |
| 5 | **钩子扩展** | 协议变体通过钩子处理，钩子可访问 PipelineContext |
| 6 | **流水线处理** | 数据处理阶段清晰，便于调试和扩展 |
| 7 | **直通优化** | 格式相同时跳过解包封包，提升性能 |
| 8 | **错误规范化** | 平台错误统一转换，携带重试/降级信号 |
| 9 | **模型解析** | 支持别名解析和能力检测 |
| 10 | **URL 上下文** | URL 构建支持认证类型和操作类型依赖 |
| 11 | **扩展参数** | PrimitiveRequest 支持平台特定参数透传 |
| 12 | **流式双轨** | 流式请求同时支持 URL 层和 JSON Body 层标识 |
| 13 | **预设示例** | 每个预设平台提供独立 examples 目录，便于测试验证 |
| 14 | **自动测试** | 修改核心模块时必须测试受影响的预设平台 |

---

## 12. 迁移计划

### 12.1 Phase 1：基础架构

1. 定义核心 traits（Site, Protocol, Authenticator）
2. 实现 Pipeline 框架
3. 实现 Primitive 原语
4. 实现 StandardError 和错误规范化

### 12.2 Phase 2：协议实现

1. 实现三种基础协议（OpenAI, Claude, Gemini）
2. 实现协议钩子机制
3. 实现错误解包方法
4. 编写协议单元测试

### 12.3 Phase 3：认证实现

1. 迁移现有认证代码
2. 实现 Authenticator trait
3. 测试各平台认证流程

### 12.4 Phase 4：预设平台

1. 迁移现有 Provider 为预设
2. 实现注册表
3. 实现 ModelResolver
4. 编写集成测试

### 12.5 Phase 5：客户端 API

1. 实现 LlmClient builder
2. 实现 from_preset 便捷方法
3. 编写使用文档

---

## 13. 架构评审要点总结

### 13.1 Site/Auth 耦合问题

**问题**：部分平台（如 Vertex AI）的 URL 结构依赖认证类型（SA vs API Key 有不同端点）。

**解决方案**：`build_url` 方法接收 `UrlContext`，包含 `auth_type` 和 `action` 信息，Site 实现根据上下文动态构建 URL。

### 13.2 流式/非流式端点异构问题

**问题**：部分平台的流式和非流式使用不同端点路径。

**解决方案**：在 `UrlContext` 中添加 `Action` 枚举，`build_url` 根据 action 返回对应端点。

### 13.3 错误规范化问题

**问题**：各平台错误格式不一，上层难以统一处理。

**解决方案**：在 `ProtocolFormat` trait 中添加 `unpack_error` 方法，将平台错误转换为 `StandardError`，携带 `retryable` 和 `fallback_hint` 信息。

### 13.4 模型别名和能力检测问题

**问题**：用户可能使用别名（如 "gpt4"），且需要知道模型能力（如是否支持 Vision）。

**解决方案**：引入 `ModelResolver` trait，负责别名解析和能力检测。

### 13.5 平台特定参数问题

**问题**：部分平台有特殊参数（如 OpenRouter 的 provider 字段）。

**解决方案**：在 `PrimitiveRequest` 中添加 `extra: HashMap<String, Value>` 字段，封包时合并到请求体中。

---

*本文档作为 `nl_llm` 模块的完整设计规范 v2.1，指导后续实现开发。*
