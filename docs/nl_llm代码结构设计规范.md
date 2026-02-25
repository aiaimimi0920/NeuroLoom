# nl_llm 代码结构设计规范 v2.3

## 概述

本文档定义了 `nl_llm` 模块的完整架构设计。采用**四维正交分解**架构，实现站点、协议、认证、模型的自由组合。

### 核心原则

1. **四维正交分解**：Site（站点）、Protocol（协议）、Auth（认证）、Model（模型）四个维度完全独立
2. **预设 + 组装双层 API**：提供开箱即用的预设平台，同时支持灵活自定义组装
3. **原语中间层**：Primitive 作为统一中间表示，解耦输入解析和输出生成
4. **流水线处理**：数据原语化 → 封包 → 认证 → 发送 → 解包
5. **错误规范化**：平台错误统一转换为标准错误，携带重试/降级信号
6. **运行时指标**：响应时间统计、并发控制、余额查询等运行时能力

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

        // 配置上下文长度
        inner.extend_context_lengths(vec![
            ("qwen3-max", 128_000),
            ("glm-4", 128_000),
            ("deepseek-r1", 64_000),
        ]);

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
| Vertex API | Gemini | API Key | Vertex 简化 API Key 端点 |
| DeepSeek | OpenAI | API Key | 余额查询 API |
| Moonshot (Kimi) | OpenAI | API Key | 无 |
| Qwen (通义千问) | OpenAI | API Key | 无 |
| Kimi | OpenAI | API Key | 无 |
| 智谱 BigModel (GLM国内版) | OpenAI | API Key | 余额查询 API |
| Z.AI (GLM海外版) | OpenAI | API Key | 动态模型列表 + 余额查询 |
| Amp CLI | OpenAI | API Key | Provider 路由 URL |
| Codex OAuth | OpenAI | OAuth | OAuth 认证流 |
| Codex API | OpenAI | API Key | 无 |
| iFlow | OpenAI | Cookie | Thinking 钩子 |
| OpenRouter | OpenAI | API Key | Provider 字段 |
| Gemini CLI | CloudCode | OAuth | CloudCode 包装 |
| Antigravity | CloudCode | OAuth | CloudCode 包装，支持 Claude 模型路由 |
| DMXAPI | OpenAI | API Key | 聚合平台，支持 Claude/GPT 模型 |
| Cubence | OpenAI | API Key | AI 工具代理平台，支持 Claude Code/Codex/Gemini CLI |
| RightCode | OpenAI | API Key | 企业级 AI Agent 中转平台，GPT-5/Codex 系列 |
| Azure OpenAI | OpenAI | API Key | 微软云平台 OpenAI 服务，需要 endpoint + deployment |

---

## 9. 扩展能力 (Extension API)

针对每个平台特有的 API 功能（例如获取账户额度与可用的模型列表），通过 `ProviderExtension` 特征作为扩展点提供支持。能够在底层直接发起经过良好封装的平台管理 HTTP 请求。

```rust
use async_trait::async_trait;
use crate::auth::traits::Authenticator;
use crate::provider::balance::BalanceStatus;

pub struct ModelInfo {
    pub id: String,
    pub description: String,
}

#[async_trait]
pub trait ProviderExtension: Send + Sync {
    fn id(&self) -> &str;
    async fn list_models(&self, http: &reqwest::Client, auth: &mut dyn Authenticator) -> anyhow::Result<Vec<ModelInfo>>;

    /// 获取账户余额或额度信息
    /// 返回 None 表示该平台不支持余额查询
    /// 返回 Some(BalanceStatus) 表示结构化的余额状态
    async fn get_balance(&self, http: &reqwest::Client, auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        Ok(None)
    }

    /// 获取并发配置
    /// 返回该平台的官方最大并发数和推荐配置
    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::default()
    }
}
```

- **挂载方式**：
  在 `ClientBuilder` 阶段可通过 `with_extension(Arc::new(IFlowExtension {}))` 注入。
- **调用方式**：
  在 `LlmClient` 构建完毕后直接使用：
  ```rust
  let models = client.list_models().await?;
  let balance = client.get_balance().await?;
  ```

### 9.1 余额查询设计

#### 计费模式分析

各 LLM 平台的计费模式主要分为以下几类：

| 计费模式 | 说明 | 示例平台 |
|----------|------|----------|
| **Token 计费** | 按输入/输出 token 数量计费，最常见 | OpenAI, DeepSeek, Claude |
| **请求次数计费** | 按 RPD (Requests Per Day) 或 RPM 限制 | Gemini Free Tier |
| **金额余额** | 充值金额逐次扣减，支持多币种 | 多数付费平台 |
| **预留吞吐量** | 按小时��费，买断算力（企业级，无免费额度概念） | Azure PTU, AWS Bedrock |

> **注意**：预留吞吐量 (PTU) 是企业级付费服务，不存在"免费额度耗尽"的问题，不在本设计考虑范围内。

#### 余额类型

| 类型 | 特点 | 重置策略 |
|------|------|----------|
| **免费额度** | 平台赠送，每日/每月重置或一次性 | 定时重置 |
| **赠送余额** | 活动赠送，无重置 | 不重置 |
| **付费余额** | 用户充值，支持负余额 | 不重置 |

#### 设计原则

- **Provider 层只负责"提供信息"**：查询并返回结构化余额数据
- **决策逻辑由上层处理**：何时降优先级、暂停调用等由调度层决定

#### 数据结构定义

```rust
/// 计费单位
#[derive(Debug, Clone)]
pub enum BillingUnit {
    /// Token 数量（最常见）
    Tokens,
    /// 请求次数 (RPD/RPM)
    Requests,
    /// 金额（美元、人民币等）
    Money { currency: String },
}

/// 额度状态（单个额度）
#[derive(Debug, Clone)]
pub struct QuotaStatus {
    /// 计费单位
    pub unit: BillingUnit,

    /// 已使用量
    pub used: f64,

    /// 总量限制（None = 无限制）
    pub total: Option<f64>,

    /// 剩余量
    pub remaining: Option<f64>,

    /// 剩余比例 (0.0-1.0)，未知则为 None
    /// - 1.0 = 满额
    /// - 0.0 = 耗尽
    pub remaining_ratio: Option<f32>,

    /// 是否会自动重置（如每日/每月）
    pub resets: bool,

    /// 重置时间
    pub reset_at: Option<DateTime<Utc>>,
}

/// 额度类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaType {
    /// 仅免费额度
    FreeOnly,
    /// 仅付费余额
    PaidOnly,
    /// 混合（有免费也有付费）
    Mixed,
    /// 未知/不支持查询
    Unknown,
}

/// 余额状态（整体）
#[derive(Debug, Clone)]
pub struct BalanceStatus {
    /// 可读描述（用于日志/显示）
    /// 例如："免费额度: 800/1000 tokens" 或 "余额: $12.34"
    pub display: String,

    /// 额度类型
    pub quota_type: QuotaType,

    /// 免费额度状态（如果有）
    pub free: Option<QuotaStatus>,

    /// 付费余额状态（如果有）
    pub paid: Option<QuotaStatus>,

    /// 是否还有可用免费额度
    /// 便捷字段，供上层快速判断是否还能"白嫖"
    pub has_free_quota: bool,

    /// 是否应该降低优先级
    /// 由各平台实现决定阈值逻辑，如：
    /// - DeepSeek: 赠送余额 < 10% 时返回 true
    /// - Gemini: RPD 接近上限时返回 true
    pub should_deprioritize: bool,

    /// 是否完全不可用
    /// - API 错误导致无法查询
    /// - 余额完全耗尽且无重置机制
    pub is_unavailable: bool,
}

impl BalanceStatus {
    /// 创建一个不支持余额查询的状态
    pub fn unsupported() -> Self { /* ... */ }

    /// 创建一个查询失败的状态
    pub fn error(message: impl Into<String>) -> Self { /* ... */ }
}
```

#### 实现示例

```rust
// DeepSeek 实现：区分赠送余额和充值余额
impl ProviderExtension for DeepSeekExtension {
    async fn get_balance(&self, http: &Client, auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        let resp = http.get("https://api.deepseek.com/user/balance")
            .bearer_auth(auth.api_key())
            .send().await?;

        let json: DeepSeekBalanceResponse = resp.json().await?;

        // 解析赠送余额和充值余额
        let granted = json.balance_infos.iter()
            .map(|i| i.granted_balance.parse::<f64>().unwrap_or(0.0))
            .sum::<f64>();
        let topped_up = json.balance_infos.iter()
            .map(|i| i.topped_up_balance.parse::<f64>().unwrap_or(0.0))
            .sum::<f64>();

        Ok(Some(BalanceStatus {
            display: format!("总额: ¥{:.2} (赠送: ¥{:.2}, 充值: ¥{:.2})",
                granted + topped_up, granted, topped_up),
            quota_type: if granted > 0.0 && topped_up > 0.0 {
                QuotaType::Mixed
            } else if granted > 0.0 {
                QuotaType::FreeOnly
            } else {
                QuotaType::PaidOnly
            },
            free: if granted > 0.0 {
                Some(QuotaStatus {
                    unit: BillingUnit::Money { currency: "CNY".into() },
                    used: 0.0,
                    total: None,
                    remaining: Some(granted),
                    remaining_ratio: None,
                    resets: false,
                    reset_at: None,
                })
            } else { None },
            paid: if topped_up > 0.0 {
                Some(QuotaStatus {
                    unit: BillingUnit::Money { currency: "CNY".into() },
                    used: 0.0,
                    total: None,
                    remaining: Some(topped_up),
                    remaining_ratio: None,
                    resets: false,
                    reset_at: None,
                })
            } else { None },
            has_free_quota: granted > 0.0,
            should_deprioritize: granted > 0.0 && granted < 1.0, // 赠送余额低于阈值
            is_unavailable: false,
        }))
    }
}

// Kimi 实现：区分代金券和现金余额
impl ProviderExtension for KimiExtension {
    async fn get_balance(&self, http: &Client, auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        // Kimi 提供: available_balance, cash_balance, voucher_balance
        // 代金券 (voucher) 视为免费额度
        // ...
    }
}

// Gemini Free Tier 实现：请求次数限制
impl ProviderExtension for GeminiExtension {
    async fn get_balance(&self, http: &Client, auth: &mut dyn Authenticator) -> anyhow::Result<Option<BalanceStatus>> {
        // Gemini 官方 API 无余额查询，但 Free Tier 有 RPD 限制
        // 可返回估算的请求次数状态
        Ok(None)
    }
}
```

#### 上层调度使用

```rust
// 调度器根据余额状态决策
if let Some(balance) = client.get_balance().await? {
    // 1. 检查是否还能白嫖
    if balance.has_free_quota {
        // 优先使用该平台
    }

    // 2. 检查是否应降低优先级
    if balance.should_deprioritize {
        // 将该平台优先级调低
        scheduler.deprioritize(platform_id);
    }

    // 3. 检查是否完全不可用
    if balance.is_unavailable {
        // 暂停该平台调用
        scheduler.pause(platform_id);
    }

    // 4. 获取详细额度信息
    if let Some(free) = &balance.free {
        println!("免费额度剩余: {:.1}%", free.remaining_ratio.unwrap_or(0.0) * 100.0);
        if free.resets {
            println!("重置时间: {:?}", free.reset_at);
        }
    }
}
```

---

## 10. 并发控制

### 10.1 设计背景

各 AI 平台都有官方声称的最大并发数限制，但实际运行中：
1. 官方限制不一定等于实际可承受的并发数
2. 网络波动、服务端负载变化会影响实际可用并发
3. 需要在运行时动态调节以获得最佳吞吐量

### 10.2 并发配置

```rust
/// 并发配置（静态）
pub struct ConcurrencyConfig {
    /// 官方声称的最大并发数
    pub official_max: usize,

    /// 初始并发限制（默认为官方值的 50%）
    pub initial_limit: usize,

    /// 最小并发限制（下限）
    pub min_limit: usize,

    /// 最大并发限制（上限，通常等于官方值）
    pub max_limit: usize,

    /// 调节策略
    pub strategy: AdjustmentStrategy,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            official_max: 10,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 10,
            strategy: AdjustmentStrategy::Aimd {
                additive_increment: 1,
                multiplicative_decrease: 0.7,
            },
        }
    }
}

/// 调节策略
pub enum AdjustmentStrategy {
    /// AIMD: 加性增、乘性减（类似 TCP 拥塞控制）
    Aimd {
        /// 每次成功增加的量
        additive_increment: usize,
        /// 失败时乘以这个系数
        multiplicative_decrease: f32,
    },

    /// 基于延迟的调节
    LatencyBased {
        /// 目标延迟（毫秒）
        target_latency_ms: u64,
        /// 低于目标延迟 * 此阈值时增加
        increase_threshold: f32,
        /// 高于目标延迟 * 此阈值时减少
        decrease_threshold: f32,
    },

    /// 固定（不调节）
    Fixed,
}
```

### 10.3 并发控制器

```rust
use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use std::collections::VecDeque;
use std::time::{Instant, Duration};

/// 并发控制器运行时状态
pub struct ConcurrencyState {
    /// 当前并发限制
    current_limit: AtomicUsize,

    /// 当前活跃请求数
    active_requests: AtomicUsize,

    /// 成功请求计数
    success_count: AtomicU64,

    /// 失败请求计数
    failure_count: AtomicU64,

    /// 最近响应时间样本（滑动窗口）
    recent_latencies: RwLock<VecDeque<u64>>,

    /// 是否处于恢复模式
    recovering: AtomicBool,
}

/// 并发控制器
pub struct ConcurrencyController {
    config: ConcurrencyConfig,
    state: ConcurrencyState,
    semaphore: Arc<Semaphore>,
}

/// 失败类型
pub enum FailureType {
    /// 429 限流错误
    RateLimited,
    /// 请求超时
    Timeout,
    /// 服务端错误
    ServerError,
    /// 其他错误
    Other,
}

impl ConcurrencyController {
    pub fn new(config: ConcurrencyConfig) -> Self {
        let initial = config.initial_limit;
        Self {
            config,
            state: ConcurrencyState::new(initial),
            semaphore: Arc::new(Semaphore::new(initial)),
        }
    }

    /// 获取许可证（开始请求前调用）
    pub async fn acquire(&self) -> ConcurrencyPermit<'_> {
        let permit = self.semaphore.acquire().await.unwrap();
        self.state.active_requests.fetch_add(1, Ordering::Relaxed);
        ConcurrencyPermit {
            controller: self,
            start_time: Instant::now(),
            _permit: Some(permit),
        }
    }

    /// 报告请求成功
    pub fn report_success(&self, latency_ms: u64) {
        self.state.success_count.fetch_add(1, Ordering::Relaxed);
        self.state.add_latency_sample(latency_ms);

        if self.should_increase() {
            self.increase_limit();
        }
    }

    /// 报告请求失败
    pub fn report_failure(&self, error_type: FailureType) {
        self.state.failure_count.fetch_add(1, Ordering::Relaxed);

        let decrease_factor = match error_type {
            FailureType::RateLimited => 0.5,
            FailureType::Timeout => 0.7,
            FailureType::ServerError => 0.8,
            FailureType::Other => 0.9,
        };

        self.decrease_limit(decrease_factor);
    }

    /// 获取状态快照
    pub fn snapshot(&self) -> ConcurrencySnapshot {
        ConcurrencySnapshot {
            official_max: self.config.official_max,
            current_limit: self.state.current_limit.load(Ordering::Relaxed),
            active_requests: self.state.active_requests.load(Ordering::Relaxed),
            success_count: self.state.success_count.load(Ordering::Relaxed),
            failure_count: self.state.failure_count.load(Ordering::Relaxed),
            avg_latency_ms: self.state.average_latency(),
        }
    }

    // ... 内部方法实现
}

/// 状态快照
pub struct ConcurrencySnapshot {
    pub official_max: usize,
    pub current_limit: usize,
    pub active_requests: usize,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_latency_ms: Option<u64>,
}
```

### 10.4 许可证（RAII 模式）

```rust
/// 许可证，持有期间占用一个并发槽位
pub struct ConcurrencyPermit<'a> {
    controller: &'a ConcurrencyController,
    start_time: Instant,
    _permit: Option<tokio::sync::SemaphorePermit<'a>>,
}

impl ConcurrencyPermit<'_> {
    /// 手动报告成功（自动计算延迟）
    pub fn report_success(self) {
        let latency = self.start_time.elapsed().as_millis() as u64;
        self.controller.report_success(latency);
        // permit 自动释放
    }

    /// 手动报告失败
    pub fn report_failure(self, error_type: FailureType) {
        self.controller.report_failure(error_type);
        // permit 自动释放
    }
}
```

### 10.5 各平台官方并发限制参考

| 平台 | 免费层 RPM | 付费层 RPM | 默认并发配置 |
|------|-----------|-----------|-------------|
| OpenAI | 3 | 10,000 | `official_max: 10` |
| Claude | 5 | 1,000 | `official_max: 10` |
| Gemini | 15 | 2,000 | `official_max: 15` |
| DeepSeek | 60 | 500 | `official_max: 20` |
| 智谱 BigModel | - | - | `official_max: 10` |
| Z.AI (GLM海外版) | - | - | `official_max: 10` |
| Qwen (通义千问) | 60 | 1,000 | `official_max: 10` |
| Kimi | - | - | `official_max: 10` |
| Amp CLI | - | - | `official_max: 5` |
| iFlow | 无限制 | - | `official_max: 100` |
| OpenRouter | 取决于后端 | - | `official_max: 10` |

> 注：RPM = Requests Per Minute，并发数需要根据平均请求时长换算。

### 10.6 使用示例

```rust
// 启用并发控制
let client = LlmClient::from_preset("openai")
    .with_api_key("sk-xxx")
    .with_concurrency()  // 使用默认配置
    .build();

// 自定义并发配置
let client = LlmClient::from_preset("claude")
    .with_api_key("sk-xxx")
    .with_concurrency_config(ConcurrencyConfig {
        official_max: 100,
        initial_limit: 20,
        min_limit: 5,
        max_limit: 100,
        strategy: AdjustmentStrategy::Aimd {
            additive_increment: 2,
            multiplicative_decrease: 0.6,
        },
    })
    .build();

// 查看并发状态
if let Some(ctrl) = client.concurrency_controller() {
    let snapshot = ctrl.snapshot();
    println!("当前限制: {}/{}", snapshot.current_limit, snapshot.official_max);
    println!("活跃请求: {}", snapshot.active_requests);
}
```

---

## 11. 响应时间统计与指标

### 11.1 Pipeline 指标收集

每个请求自动收集响应时间等指标：

```rust
/// Pipeline 执行指标
pub struct PipelineMetrics {
    /// 请求开始时间
    pub start_time: Instant,

    /// 请求结束时间
    pub end_time: Option<Instant>,

    /// 总响应时间（毫秒）
    pub response_time_ms: Option<u64>,

    /// 首个 Token 时间（流式请求）
    pub first_token_time_ms: Option<u64>,

    /// 各阶段耗时
    pub stage_timings: HashMap<String, u64>,
}

impl PipelineMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            end_time: None,
            response_time_ms: None,
            first_token_time_ms: None,
            stage_timings: HashMap::new(),
        }
    }

    /// 记录阶段耗时
    pub fn record_stage(&mut self, stage: &str, duration_ms: u64) {
        self.stage_timings.insert(stage.to_string(), duration_ms);
    }

    /// 完成记录
    pub fn finish(&mut self) {
        self.end_time = Some(Instant::now());
        self.response_time_ms = Some(self.start_time.elapsed().as_millis() as u64);
    }
}
```

### 11.2 指标存储

```rust
use std::sync::RwLock;
use std::collections::VecDeque;

/// 指标存储（滑动窗口）
pub struct MetricsStore {
    /// 最近 N 次请求的指标
    recent_metrics: RwLock<VecDeque<PipelineMetrics>>,

    /// 窗口大小
    window_size: usize,

    /// 累计统计
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_latency_ms: AtomicU64,
}

impl MetricsStore {
    pub fn new(window_size: usize) -> Self {
        Self {
            recent_metrics: RwLock::new(VecDeque::with_capacity(window_size)),
            window_size,
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
        }
    }

    /// 记录指标
    pub fn record(&self, metrics: PipelineMetrics) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(
            metrics.response_time_ms.unwrap_or(0),
            Ordering::Relaxed
        );

        let mut recent = self.recent_metrics.write().unwrap();
        if recent.len() >= self.window_size {
            recent.pop_front();
        }
        recent.push_back(metrics);
    }

    /// 获取平均响应时间
    pub fn avg_latency_ms(&self) -> u64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0;
        }
        self.total_latency_ms.load(Ordering::Relaxed) / total
    }

    /// 获取统计摘要
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            avg_latency_ms: self.avg_latency_ms(),
            recent_count: self.recent_metrics.read().unwrap().len(),
        }
    }
}

/// 指标摘要
pub struct MetricsSummary {
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ms: u64,
    pub recent_count: usize,
}
```

### 11.3 集成到 LlmClient

```rust
impl LlmClient {
    pub async fn complete(&self, req: &PrimitiveRequest) -> Result<LlmResponse> {
        // 获取并发许可证
        let permit = self.concurrency.as_ref().map(|c| c.acquire()).transpose()?;

        // 创建指标记录
        let mut metrics = PipelineMetrics::new();

        // 执行请求
        let result = self.inner_complete(req).await;

        // 记录结果
        metrics.finish();
        self.metrics.record(metrics);

        // 报告并发状态
        if let Some(permit) = permit {
            match &result {
                Ok(_) => permit.report_success(),
                Err(e) => permit.report_failure(e.into()),
            }
        }

        result
    }

    /// 获取指标摘要
    pub fn metrics_summary(&self) -> MetricsSummary {
        self.metrics.summary()
    }
}
```

---

## 12. 目录结构

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
│   ├── error.rs             # StandardError 定义
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
│   ├── context.rs           # UrlContext, Action 定义
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
├── provider/                 # 供应商扩展能力
│   ├── mod.rs
│   ├── extension.rs         # ProviderExtension trait
│   ├── openai.rs            # OpenAI 扩展
│   ├── anthropic.rs         # Anthropic 扩展
│   ├── gemini.rs            # Gemini 扩展
│   ├── gemini_cli.rs        # Gemini CLI 扩展
│   ├── vertex.rs            # Vertex AI 扩展
│   ├── deepseek.rs          # DeepSeek 扩展（含余额查询）
│   ├── zhipu.rs             # 智谱 BigModel 扩展（含余额查询）
│   ├── zai.rs               # Z.AI 扩展（动态模型列表 + 余额查询）
│   ├── qwen.rs              # 通义千问扩展
│   ├── kimi.rs              # Kimi 扩展
│   ├── moonshot.rs          # Moonshot 扩展
│   ├── codex.rs             # Codex 扩展
│   ├── amp.rs               # Amp CLI 扩展
│   ├── iflow.rs             # iFlow 扩展
│   └── antigravity.rs       # Antigravity 扩展
│
├── model/                    # 模型解析
│   ├── mod.rs
│   ├── resolver.rs          # ModelResolver trait
│   ├── capabilities.rs      # Capability flags
│   └── default.rs           # DefaultModelResolver
│
├── concurrency/              # 【新增】并发控制
│   ├── mod.rs
│   ├── config.rs            # ConcurrencyConfig
│   ├── controller.rs        # ConcurrencyController
│   └── permit.rs            # ConcurrencyPermit
│
├── metrics/                  # 【新增】指标收集
│   ├── mod.rs
│   ├── pipeline.rs          # PipelineMetrics
│   └── store.rs             # MetricsStore
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

### 12.1 预设平台示例目录

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
│   ├── chat/
│   ├── stream/
│   ├── models/
│   └── auth/
│
├── zhipu/                   # 智谱 BigModel（国内版）
│   ├── chat/
│   ├── stream/
│   ├── models/
│   └── auth/
│
├── zai/                     # Z.AI（GLM 海外版）
│   ├── chat/
│   ├── stream/
│   ├── models/
│   └── auth/
│
├── amp/                     # Amp CLI
│   ├── chat/
│   ├── stream/
│   ├── models/
│   └── auth/
│
├── moonshot/
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

### 12.2 自动测试要求

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

## 13. 使用示例

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

### 10.6 使用并发控制

```rust
// 启用并发控制（使用默认配置）
let client = LlmClient::from_preset("openai")
    .with_api_key("sk-xxx")
    .with_concurrency()  // 使用 Extension 中配置的默认值
    .build();

// 自定义并发配置
let client = LlmClient::from_preset("claude")
    .with_api_key("sk-xxx")
    .with_concurrency_config(ConcurrencyConfig {
        official_max: 100,
        initial_limit: 20,
        min_limit: 5,
        max_limit: 100,
        strategy: AdjustmentStrategy::Aimd {
            additive_increment: 2,
            multiplicative_decrease: 0.6,
        },
    })
    .build();

// 查看并发状态
if let Some(ctrl) = client.concurrency_controller() {
    let snapshot = ctrl.snapshot();
    println!("当前限制: {}/{}", snapshot.current_limit, snapshot.official_max);
    println!("活跃请求: {}", snapshot.active_requests);
    println!("平均延迟: {:?}ms", snapshot.avg_latency_ms);
}
```

### 10.7 使用余额查询

```rust
let client = LlmClient::from_preset("openai")
    .with_api_key("sk-xxx")
    .build();

// 查询余额
if let Some(balance) = client.get_balance().await? {
    println!("账户余额: {}", balance);
} else {
    println!("该平台不支持余额查询");
}
```

### 10.8 查看运行时指标

```rust
let client = LlmClient::from_preset("openai")
    .with_api_key("sk-xxx")
    .with_concurrency()
    .build();

// 执行一些请求...
for i in 0..10 {
    let response = client.complete(&PrimitiveRequest::single_user_message("Hello")).await?;
}

// 查看指标摘要
let summary = client.metrics_summary();
println!("总请求数: {}", summary.total_requests);
println!("平均延迟: {}ms", summary.avg_latency_ms);
println!("错误率: {:.2}%",
    summary.total_errors as f64 / summary.total_requests as f64 * 100.0);
```

---

## 14. 设计原则总结

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
| 15 | **并发控制** | 官方并发数 + AIMD 弹性调节，自动适应平台限流 |
| 16 | **指标收集** | 自动收集响应时间、成功率等运行时指标 |
| 17 | **余额查询** | Extension 支持各平台余额/额度查询 |

---

## 15. 迁移计划

### 14.1 Phase 1：基础架构

1. 定义核心 traits（Site, Protocol, Authenticator）
2. 实现 Pipeline 框架
3. 实现 Primitive 原语
4. 实现 StandardError 和错误规范化

### 14.2 Phase 2：协议实现

1. 实现三种基础协议（OpenAI, Claude, Gemini）
2. 实现协议钩子机制
3. 实现错误解包方法
4. 编写协议单元测试

### 14.3 Phase 3：认证实现

1. 迁移现有认证代码
2. 实现 Authenticator trait
3. 测试各平台认证流程

### 14.4 Phase 4：预设平台

1. 迁移现有 Provider ��预设
2. 实现注册表
3. 实现 ModelResolver
4. 编写集成测试

### 14.5 Phase 5：客户端 API

1. 实现 LlmClient builder
2. 实现 from_preset 便捷方法
3. 编写使用文档

### 14.6 Phase 6：运行时能力（新增）

1. 实现并发控制器（ConcurrencyController）
2. 实现 Pipeline 指标收集（PipelineMetrics）
3. 实现指标存储（MetricsStore）
4. 为各平台实现 `get_balance` 方法
5. 为各平台配置官方并发数
6. 编写运行时能力测试

---

## 16. 架构评审要点总结

### 15.1 Site/Auth 耦合问题

**问题**：部分平台（如 Vertex AI）的 URL 结构依赖认证类型（SA vs API Key 有不同端点）。

**解决方案**：`build_url` 方法接收 `UrlContext`，包含 `auth_type` 和 `action` 信息，Site 实现根据上下文动态构建 URL。

### 15.2 流式/非流式端点异构问题

**问题**：部分平台的流式和非流式使用不同端点路径。

**解决方案**：在 `UrlContext` 中添加 `Action` 枚举，`build_url` 根据 action 返回对应端点。

### 15.3 错误规范化问题

**问题**：各平台错误格式不一，上层难以统一处理。

**解决方案**：在 `ProtocolFormat` trait 中添加 `unpack_error` 方法，将平台错误转换为 `StandardError`，携带 `retryable` 和 `fallback_hint` 信息。

### 15.4 模型别名和能力检测问题

**问题**：用户可能使用别名（如 "gpt4"），且需要知道模型能力（如是否支持 Vision）。

**解决方案**：引入 `ModelResolver` trait，负责别名解析和能力检测。

### 15.5 平台特定参数问题

**问题**：部分平台有特殊参数（如 OpenRouter 的 provider 字段）。

**解决方案**：在 `PrimitiveRequest` 中添加 `extra: HashMap<String, Value>` 字段，封包时合并到请求体中。

### 15.6 并发控制问题（新增）

**问题**：官方并发限制不一定等于实际可用并发，需要在运行时弹性调节。

**解决方案**：引入 `ConcurrencyController`，采用 AIMD（加性增、乘性减）算法，类似 TCP 拥塞控制：
- 成功时逐步增加并发限制
- 遇到 429/超时时降低并发限制
- 自动适应平台实际负载能力

---

*本文档作为 `nl_llm` 模块的完整设计规范 v2.3，指导后续实现开发。*
