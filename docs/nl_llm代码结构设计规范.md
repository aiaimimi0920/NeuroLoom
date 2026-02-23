# nl_llm 代码结构设计规范

## 概述

本文档定义了 `nl_llm` 模块的完整架构设计，包含代码结构、原语格式、转换层、容错机制和黑魔法代理等所有方面。基于以下核心原则：

1. **认证与协议分离**：认证方式（OAuth/API Key/Service Account）与请求协议（Claude/OpenAI/Gemini）正交
2. **原语中间层**：引入 Primitive 作为统一中间表示，解耦输入解析和输出生成
3. **协议优先**：Provider 按协议划分，官方/兼容站由配置参数决定
4. **配置区分类型**：`base_url` 是否自定义决定官方还是转发站，不是类型差异
5. **分层容错**：Provider 层处理特有错误，Gateway 层处理通用容错

---

## 1. 认证模式分析

### 1.1 三种认证类型

| 认证类型 | 特点 | Provider 示例 |
|----------|------|---------------|
| **OAuth** | 需浏览器登录、Token 会过期、需定期刷新 | Claude OAuth、Gemini CLI、Antigravity、iFlow |
| **API Key** | 直接使用、不过期、按量计费 | 所有 Provider 都支持 |
| **Service Account** | JSON 凭据、JWT 认证、GCP 专用 | Vertex AI |

### 1.2 官方 vs 转发站

**区分关键：`base_url` 配置**

```yaml
# 官方端点（不设置 base_url）
- provider: claude
  api_key: "sk-ant-xxx"

# 转发站（设置 base_url）
- provider: claude
  api_key: "sk-xxx"
  base_url: "https://openrouter.ai/api/v1"
```

**结论：官方/转发站只是配置差异，不是类型差异。**

### 1.3 OAuth 实现差异

各 Provider 的 OAuth 实现差异巨大，无法共用核心逻辑：

| 维度 | Claude OAuth | Gemini CLI OAuth | Antigravity OAuth | iFlow |
|------|-------------|------------------|-------------------|-------|
| PKCE | ✅ 需要 | ❌ 不需要 | ❌ 不需要 | N/A |
| 回调端口 | 54545 | 8085 | 51121 | N/A |
| Token 端点 | Anthropic | Google | Google | N/A |
| Scopes | 3 个 | 3 个 | 5 个 | N/A |
| Client Metadata | 无 | 逗号分隔 | JSON 格式 | N/A |

**结论：OAuth 认证必须各 Provider 独立实现，只能共享工具函数。**

### 1.4 网关协议的正交分解体系 (Orthogonal Decomposition)

随着大模型网关的复杂化，传统的“一个 Provider 包打天下”的设计反模式（Anti-Pattern）已被废弃。新的架构推演明确了客户端网关由三个**完全正交（互不影响）**的积木构成：

1. **认证维 (Auth / 换 Key)**：唯一的信条是将任意外部凭证（OAuth 网页令牌、iFlow Cookie、GCP JWT）映射转换为最终发送请求时的合法字符串（`Authorization: Bearer` 或 `x-goog-api-key` 头）。在发送前，它们都是等价的合法 Token。
2. **路由维 (Endpoint / 通道)**：请求的目标服务器地址（例如：官方的 `api.anthropic.com` 或是本地代理 `127.0.0.1:8080`）。通道自身是透明的，只负责网络连接和扣费拦截。
3. **协议维 (Protocol / 压包解包)**：数据 Payload 的最终形状（由于模型方言存在差异产生的结构区别：OpenAI 格式、Gemini 结构、Claude 结构）。

#### 1.4.1 协议二维决定论 (Protocol 2D Determinism)

协议（发什么格式的 JSON 包）并不总是跟随网关走的，而是由 `f(平台, 模型)` 两个维度共同决定的：

- **平台霸权型（只认平台）**：例如 iFlow、OpenRouter。此类代理强行包裹了一层翻译中间件。无论你要调用 `gemini-1.5-pro` 还是 `claude-3-opus`，**所有请求必须一律使用标准 OpenAI Protocol 压包**。发往通道后，服务端自行“解包 -> 翻译为对应模型方言请求 -> 收集结果 -> 打包回 OpenAI 格式返回”。
- **平台透传型（强绑定模型）**：例如 Google Vertex AI，平台仅仅提供一条专属验证光纤。如果通过 Vertex 调 `gemini`，客户端需要压包为 Gemini 原生格式（`contents` 数组）；如果在**同一个网关同一套代码下**调 `claude`，平台拒绝翻译，客户端必须动态切回 Anthropic 官方格式（`messages` 数组）进行压包。

因此，**协议必须彻底脱离认证和请求路由独立存在**，成为可即插即拔的转换层（这由我们 `translator/` 目录中的 Protocol Unwrapper/Wrapper 强力保证）。

#### 1.4.2 极简组装形态 (The Ultimate Architecture)

高度去耦后的 LlmClient 将不再需要编写数十张功能相似的宏伟 Provider 面条代码，调用将被收缩为极具工业机械感的积木按需拼接调用：

```rust
// 伪代码示例：调用 iFlow 代理站背后的 Claude 模型
let client = LlmClient::new(
    Endpoint::Custom("https://iflow-proxy.com/v1"),       // (路由维) 去哪个地址
    Auth::IFlowCookie("BXAuth=..."),                      // (认证维) 怎么换门票
    ProtocolPicker::select("iflow", "claude-3.5-sonnet")  // (协议维) 用哪种信封 => 生成 OpenAI Protocol
);
```
此理念构筑了项目 `Proxy` 以及 `Translator` 管道组件彻底剥离的哲学基石。

---

## 2. 目录结构

```
nl_llm/src/
├── lib.rs                        # 模块入口
│
├── auth/                         # 认证层
│   ├── mod.rs                    # 模块导出
│   ├── types.rs                  # 共享类型定义
│   ├── storage.rs                # Token 持久化工具
│   └── providers/                # 各 Provider 独立 OAuth 实现
│       ├── mod.rs
│       ├── claude.rs             # Claude OAuth (PKCE)
│       ├── gemini_cli.rs         # Gemini CLI OAuth
│       ├── antigravity.rs        # Antigravity OAuth
│       ├── iflow.rs              # iFlow Cookie→Token
│       └── vertex_sa.rs          # Vertex Service Account
│
├── primitive/                    # 原语格式（中间表示）
│   ├── mod.rs
│   ├── request.rs                # PrimitiveRequest
│   ├── message.rs                # PrimitiveMessage, PrimitiveContent
│   ├── tool.rs                   # PrimitiveTool
│   ├── parameters.rs             # PrimitiveParameters
│   └── metadata.rs               # PrimitiveMetadata
│
├── translator/                   # 转换层
│   ├── mod.rs
│   ├── format.rs                 # Format 枚举
│   ├── wrapper.rs                # WrapperKind 枚举
│   ├── pipeline.rs               # TranslatorPipeline
│   ├── detector.rs               # 格式和包裹检测
│   ├── error.rs                  # TranslateError
│   ├── unwrapper/                # 解包器
│   │   ├── mod.rs
│   │   ├── trait.rs              # Unwrapper trait
│   │   ├── claude.rs
│   │   ├── openai.rs
│   │   ├── gemini.rs
│   │   ├── gemini_cli.rs
│   │   ├── codex.rs
│   │   └── antigravity.rs
│   └── wrapper/                  # 封装器
│       ├── mod.rs
│       ├── trait.rs              # Wrapper trait
│       ├── claude.rs
│       ├── openai.rs
│       ├── gemini.rs
│       ├── gemini_cli.rs
│       ├── codex.rs
│       └── antigravity.rs
│
├── provider/                     # Provider 层（按协议划分）
│   ├── mod.rs
│   ├── traits.rs                 # LlmProvider trait
│   │
│   ├── claude/                   # Claude 协议
│   │   ├── mod.rs
│   │   ├── config.rs             # ClaudeConfig, ClaudeAuth
│   │   ├── compiler.rs           # 原语 → Claude JSON
│   │   └── provider.rs           # 统一处理官方/兼容站
│   │
│   ├── openai/                   # OpenAI 协议
│   │   ├── mod.rs
│   │   ├── config.rs             # OpenAIConfig
│   │   ├── compiler.rs           # 原语 → OpenAI JSON
│   │   └── provider.rs           # 统一处理官方/兼容站
│   │
│   ├── gemini/                   # Gemini 协议
│   │   ├── mod.rs
│   │   ├── common.rs             # 共享编译逻辑
│   │   ├── config.rs             # GeminiConfig（API Key 认证）
│   │   ├── provider.rs           # API Key 认证（官方/兼容站）
│   │   ├── vertex.rs             # Service Account 认证
│   ├── gemini_cli/               # Gemini CLI 协议（从 gemini 拆分）
│   │   ├── mod.rs
│   │   ├── config.rs
│   │   └── provider.rs
│   │
│   ├── antigravity/              # Antigravity 协议（独立于 gemini）
│   │   ├── mod.rs
│   │   ├── config.rs
│   │   └── provider.rs
│   │
│   ├── codex/                    # Codex 协议
│   │   ├── mod.rs
│   │   ├── config.rs
│   │   ├── compiler.rs
│   │   └── provider.rs
│   │
│   └── iflow/                    # iFlow（Cookie 认证）
│       ├── mod.rs
│       ├── config.rs
│       └── provider.rs
│
├── black_magic_proxy/            # 黑魔法代理层
│   ├── mod.rs                    # 模块导出
│   ├── catalog.rs                # 项目 profile 目录
│   ├── client.rs                 # 统一调用准备器
│   └── types.rs                  # ProxyExposureKind 等类型
│
├── gateway.rs                    # Gateway 编排层
├── fallback.rs                   # 降级路由
├── token_bucket.rs               # 令牌桶限流
├── prompt_ast.rs                 # Prompt AST
└── prompt.rs                     # Prompt 构建器
```

---

## 3. 核心类型定义

### 3.1 认证层 (auth/)

#### types.rs - 共享类型

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 认证类型（顶层枚举）
#[derive(Debug, Clone)]
pub enum Auth {
    /// API Key 认证
    ApiKey(ApiKeyConfig),

    /// OAuth 认证
    OAuth {
        provider: OAuthProvider,
        token_path: PathBuf,
    },

    /// Service Account 认证
    ServiceAccount {
        provider: SAProvider,
        credentials_json: String,
    },
}

/// API Key 配置（统一结构）
///
/// 设计说明：
/// - API Key 本质上只是一个字符串，不区分官方/转发站
/// - 区分的关键是 `base_url`，这只是配置参数
/// - 真正决定请求格式的是 `provider` 字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API Key 字符串
    pub key: String,

    /// 自定义 Base URL
    /// - None: 使用官方端点
    /// - Some(url): 使用转发站/代理
    pub base_url: Option<String>,

    /// Provider 标识（用于选择请求格式）
    pub provider: ApiKeyProvider,
}

impl ApiKeyConfig {
    /// 是否为官方端点
    pub fn is_official(&self) -> bool {
        self.base_url.is_none()
    }

    /// 获取 Base URL（官方或自定义）
    pub fn base_url_or<'a>(&self, default: &'a str) -> Cow<'a, str> {
        match &self.base_url {
            Some(url) => Cow::Owned(url.clone()),
            None => Cow::Borrowed(default),
        }
    }
}

/// API Key Provider 标识
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ApiKeyProvider {
    Anthropic,
    OpenAI,
    GeminiAIStudio,
    Codex,
}

/// OAuth Provider 标识
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OAuthProvider {
    Claude,
    GeminiCli,
    Antigravity,
}

/// Service Account Provider 标识
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SAProvider {
    VertexAI,
}

/// Token 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStatus {
    Valid,
    ExpiringSoon,
    Expired,
    RefreshFailed,
}

/// 通用 Token 存储格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStorage {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub email: Option<String>,
    pub provider: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TokenStorage {
    /// 检查 Token 状态
    pub fn status(&self, lead_seconds: i64) -> TokenStatus {
        let Some(expires_at) = self.expires_at else {
            return TokenStatus::Valid;
        };

        let now = Utc::now();
        let threshold = expires_at - chrono::Duration::seconds(lead_seconds);

        if now >= expires_at {
            TokenStatus::Expired
        } else if now >= threshold {
            TokenStatus::ExpiringSoon
        } else {
            TokenStatus::Valid
        }
    }
}
```

#### TokenStorage 向后兼容解析机制
由于 `TokenStorage` 的演化（如引入了必需的 `provider` 字段取代了专有 Struct），各个 Provider（GeminiCLI / Antigravity / IFlow）的 `from_file()` 加载逻辑中包含**容灾解析机制**：如果遭遇 JSON Payload 结构不匹配或缺失字段的旧缓存文件，解析系统会优雅地捕获 `serde_json::Error`，丢弃损坏的内存实例，并强制触发一次平滑的重新认证或 Cookie 重新注入流程，而不能引发运行时崩溃。

#### providers/claude.rs - Claude OAuth 独立实现

```rust
use crate::auth::{TokenStorage, TokenStatus};

/// Claude OAuth 配置（Claude 特有常量）
pub const CLAUDE_OAUTH_CONFIG: ClaudeOAuthConfig = ClaudeOAuthConfig {
    client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
    redirect_port: 54545,
    auth_url: "https://claude.ai/oauth/authorize",
    token_url: "https://console.anthropic.com/v1/oauth/token",
    scopes: &[
        "org:create_api_key",
        "user:profile",
        "user:inference",
    ],
};

/// Claude OAuth 配置
#[derive(Debug, Clone)]
pub struct ClaudeOAuthConfig {
    pub client_id: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

/// Claude PKCE 挑战码（Claude 特有）
#[derive(Debug, Clone)]
pub struct ClaudePkceCodes {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// Claude OAuth 客户端
pub struct ClaudeOAuth {
    config: ClaudeOAuthConfig,
    storage: Option<TokenStorage>,
    http: reqwest::Client,
}

impl ClaudeOAuth {
    pub fn new() -> Self { ... }

    pub fn from_file(path: &Path) -> crate::Result<Self> { ... }

    /// 生成 PKCE 挑战码（Claude 特有）
    pub fn generate_pkce(&self) -> ClaudePkceCodes { ... }

    /// 生成授权 URL
    pub fn build_auth_url(&self, state: &str, pkce: &ClaudePkceCodes) -> String { ... }

    /// 用授权码换取 Token
    pub async fn exchange_code(
        &mut self,
        code: &str,
        pkce: &ClaudePkceCodes
    ) -> crate::Result<()> { ... }

    /// 刷新 Token
    pub async fn refresh_token(&mut self) -> crate::Result<()> { ... }

    /// 获取 Access Token
    pub fn access_token(&self) -> Option<&str> {
        self.storage.as_ref().map(|s| s.access_token.as_str())
    }

    /// 是否需要刷新
    pub fn needs_refresh(&self) -> bool {
        self.storage.as_ref().map_or(true, |s| {
            matches!(s.status(300), TokenStatus::Expired | TokenStatus::ExpiringSoon)
        })
    }
}
```

### 3.2 原语格式 (primitive/)

#### 设计哲学

采用**编译器式中间表示（IR）架构**，避免 N×(N-1) 组合爆炸：

```
输入端                    中间层                    输出端

┌──────────┐         ┌──────────┐         ┌──────────┐
│ Claude   │──┐      │          │      ┌──│ Claude   │
│ Code     │  │      │          │      │  │          │
└──────────┘  │      │          │      │  └──────────┘
              │      │          │      │
┌──────────┐  │      │          │      │  ┌──────────┐
│ OpenAI   │──┼─→ 解包 →│  原语    │→ 封装 ─┼→│ OpenAI   │
│ Client   │  │ (Unwrap)│   (IR)   │ (Wrap) │  │ Compat   │
└──────────┘  │      │          │      │  └──────────┘
              │      │          │      │
┌──────────┐  │      │          │      │  ┌──────────┐
│ Gemini   │──┘      │          │      └──│ Gemini   │
│ Client   │         └──────────┘         │          │
└──────────┘                              └──────────┘

快速路径: 输入格式 === 输出格式 ────────────────────→ 直通
```

与现有 Prompt AST 的关系（双轨体系）：

| 组件 | 用途 | 层次 |
|------|------|------|
| **Prompt AST** | 系统内部认知层的提示词抽象 | 认知层 → 模型 |
| **Primitive (IR)** | 外部客户端请求的协议转换 | 客户端 → 网关 |

#### request.rs

```rust
use serde::{Deserialize, Serialize};

/// 中间原语格式 - 与任何特定 API 无关的抽象表示
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrimitiveRequest {
    /// 模型标识（不含 provider 前缀）
    pub model: String,

    /// 系统提示词（已解包，纯用户意图）
    pub system: Option<String>,

    /// 消息历史（已标准化）
    pub messages: Vec<PrimitiveMessage>,

    /// 工具定义（已标准化，已过滤内置工具）
    pub tools: Vec<PrimitiveTool>,

    /// 生成参数
    #[serde(flatten)]
    pub parameters: PrimitiveParameters,

    /// 元数据（用于追踪和调试）
    #[serde(skip_serializing_if = "PrimitiveMetadata::is_empty")]
    pub metadata: PrimitiveMetadata,
}
```

#### message.rs

```rust
/// 标准化消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveMessage {
    pub role: Role,
    pub content: Vec<PrimitiveContent>,
}

/// 角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// 标准化内容块
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PrimitiveContent {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        mime_type: String,
        data: String,  // Base64
    },

    #[serde(rename = "tool_use")]
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_call_id: String,
        content: String,
        is_error: bool,
    },
}
```

#### tool.rs

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}
```

#### parameters.rs

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrimitiveParameters {
    pub max_tokens: Option<u64>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u64>,
}
```

#### metadata.rs

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrimitiveMetadata {
    /// 原始格式
    pub source_format: Format,
    /// 检测到的包裹类型
    pub wrapper_kind: WrapperKind,
    /// 是否被解包
    pub was_unwrapped: bool,
    /// 原始请求中的客户端特有字段（保留用于回填）
    pub client_specific: HashMap<String, serde_json::Value>,
}
```

### 3.3 转换层 (translator/)

#### format.rs

```rust
/// 支持的格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    OpenAI,
    OpenAIResponse,
    Claude,
    Gemini,
    GeminiCLI,
    Codex,
    Antigravity,
}
```

#### wrapper.rs - 包裹类型与检测

```rust
/// 包裹类型
///
/// 某些客户端会在请求中"包裹"额外的身份信息和工具定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapperKind {
    None,
    ClaudeCode,
    GeminiCLI,
    Codex,
    Antigravity,
}

/// 检测输入是否带有包裹
pub fn detect_wrapper(request: &serde_json::Value) -> WrapperKind {
    // Claude Code 特征：system 以特定身份开头
    if let Some(system) = request.get("system").and_then(|s| s.as_array()) {
        if system.first()
            .and_then(|s| s.get("text"))
            .and_then(|t| t.as_str())
            .map(|t| t.contains("Claude Code"))
            .unwrap_or(false)
        {
            return WrapperKind::ClaudeCode;
        }
    }

    // Gemini CLI 特征：工具名前缀
    if let Some(tools) = request.get("tools").and_then(|t| t.as_array()) {
        if tools.iter().any(|t| {
            t.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.starts_with("proxy_") || is_gemini_cli_builtin_tool(n))
                .unwrap_or(false)
        }) {
            return WrapperKind::GeminiCLI;
        }
    }

    // Codex 特征
    if request.get("instructions").is_some()
        && request.get("previous_response_id").is_some()
    {
        return WrapperKind::Codex;
    }

    // Antigravity 特征
    if let Some(system) = request.get("systemInstruction") {
        if contains_antigravity_signature(system) {
            return WrapperKind::Antigravity;
        }
    }

    WrapperKind::None
}
```

#### builtin_tools.rs - 内置工具过滤规则

解包时需要过滤各客户端自带的内置工具，只保留用户自定义工具：

```rust
/// Claude Code 内置工具
const CLAUDE_CODE_BUILTIN_TOOLS: &[&str] = &[
    "Read", "Write", "Edit", "MultiEdit",
    "Glob", "Grep", "NotebookRead", "NotebookEdit",
    "Bash", "Task", "TodoWrite",
    "WebFetch", "WebSearch",
    "KillShell", "LocalShell", "Stop",
];

/// Gemini CLI 内置工具
const GEMINI_CLI_BUILTIN_TOOLS: &[&str] = &[
    "proxy_read_file", "proxy_write_file", "proxy_edit_file",
    "proxy_list_directory", "proxy_search_files",
    "proxy_execute_command", "proxy_create_directory",
    "proxy_delete_file", "proxy_move_file", "proxy_copy_file",
    "code_execution", "web_search",
];
```

#### unwrapper/trait.rs 和 wrapper/trait.rs - 解包器与封装器

##### Unwrapper trait

```rust
/// 解包器 trait：将特定格式转换为原语
pub trait Unwrapper: Send + Sync {
    fn source_format(&self) -> Format;
    fn unwrap(&self, request: &serde_json::Value) -> Result<PrimitiveRequest>;
    fn filter_builtin_tools(&self, tools: &[serde_json::Value]) -> Vec<PrimitiveTool>;
    fn extract_user_system(&self, request: &serde_json::Value) -> Option<String>;
}
```

##### Wrapper trait

```rust
/// 封装器 trait：将原语转换为特定格式
pub trait Wrapper: Send + Sync {
    fn target_format(&self) -> Format;
    fn wrap(&self, primitive: &PrimitiveRequest) -> Result<serde_json::Value>;
    fn inject_identity(&self, system: &mut Option<String>);
    fn add_builtin_tools(&self, tools: &mut Vec<PrimitiveTool>);
}
```

#### pipeline.rs - 转换管道

```rust
pub struct TranslatorPipeline {
    source_format: Format,
    target_format: Format,
    unwrapper: Box<dyn Unwrapper>,
    wrapper: Box<dyn Wrapper>,
}

impl TranslatorPipeline {
    pub fn translate(&self, request: &[u8]) -> Result<Vec<u8>> {
        let parsed: serde_json::Value = serde_json::from_slice(request)?;

        // 快速路径：相同格式直通
        if self.source_format == self.target_format {
            return Ok(request.to_vec());
        }

        let wrapper = detect_wrapper(&parsed);
        if self.can_passthrough(wrapper) {
            return Ok(request.to_vec());
        }

        // 完整转换流程：解包 → 原语 → 封装
        let primitive = self.unwrapper.unwrap(&parsed)?;
        let output = self.wrapper.wrap(&primitive)?;
        Ok(serde_json::to_vec(&output)?)
    }

    fn can_passthrough(&self, wrapper: WrapperKind) -> bool {
        matches!(
            (wrapper, self.source_format, self.target_format),
            (WrapperKind::ClaudeCode, Format::Claude, Format::Claude) |
            (WrapperKind::GeminiCLI, Format::GeminiCLI, Format::GeminiCLI) |
            (WrapperKind::None, Format::OpenAI, Format::OpenAI)
        )
    }
}
```

#### 转换矩阵

```
                ┌─────────────────────────────────────────────────────────────────────┐
                │                        输入格式 (Source)                             │
                ├──────────┬──────────┬────────┬────────┬───────────┬───────┬──────────┤
                │  openai  │ openai-  │ claude │ gemini │ gemini-cli│ codex │antigrav- │
                │          │ response │        │        │           │       │   ity    │
    ┌───────────┼──────────┼──────────┼────────┼────────┼───────────┼───────┼──────────┤
    │ claude    │    ✅    │    ✅    │   ⚡   │   ✅   │     ✅    │  ✅   │    ✅    │
    ├───────────┼──────────┼──────────┼────────┼────────┼───────────┼───────┼──────────┤
    │ gemini    │    ✅    │    ✅    │   ✅   │   ⚡   │     ✅    │  ✅   │    ✅    │
    ├───────────┼──────────┼──────────┼────────┼────────┼───────────┼───────┼──────────┤
输  │ gemini-cli│    ✅    │    ✅    │   ✅   │   ✅   │     ⚡    │  ✅   │    ✅    │
出  ├───────────┼──────────┼──────────┼────────┼────────┼───────────┼───────┼──────────┤
格  │ openai    │    ⚡    │    ✅    │   ✅   │   ✅   │     ✅    │  ✅   │    ✅    │
式  ├───────────┼──────────┼──────────┼────────┼────────┼───────────┼───────┼──────────┤
    │ codex     │    ✅    │    ⚡    │   ✅   │   ✅   │     ✅    │  ⚡   │    ✅    │
    ├───────────┼──────────┼──────────┼────────┼────────┼───────────┼───────┼──────────┤
    │antigravity│    ✅    │    ✅    │   ✅   │   ✅   │     ✅    │  ✅   │    ⚡    │
    └───────────┴──────────┴──────────┴────────┴────────┴───────────┴───────┴──────────┘

    ✅ = 需要转换（通过原语）
    ⚡ = 可直通（相同格式）
```

#### error.rs - 转换层错误

```rust
#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("JSON 解析失败: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("不支持的格式: {0}")]
    UnsupportedFormat(String),

    #[error("缺少必要字段: {0}")]
    MissingField(String),

    #[error("无效的角色类型: {0}")]
    InvalidRole(String),

    #[error("工具转换失败: {0}")]
    ToolConversion(String),

    #[error("内容块转换失败: {0}")]
    ContentConversion(String),
}
```

### 3.4 Provider 层 (provider/)

#### traits.rs

```rust
use async_trait::async_trait;
use futures::Stream;

use crate::auth::Auth;
use crate::primitive::PrimitiveRequest;

/// LLM Provider 统一 Trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 唯一标识
    fn id(&self) -> &str;

    /// 认证类型
    fn auth(&self) -> &Auth;

    /// 支持的模型列表
    fn supported_models(&self) -> &[&str];

    /// 将原语编译为请求体
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value;

    /// 执行请求
    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse>;

    /// 流式执行
    async fn stream(
        &self,
        body: serde_json::Value
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>>;

    /// 是否需要刷新认证
    fn needs_refresh(&self) -> bool {
        false
    }

    /// 刷新认证
    async fn refresh_auth(&mut self) -> crate::Result<()> {
        Ok(())
    }
}

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub stop_reason: StopReason,
}

/// LLM 流式块
#[derive(Debug, Clone)]
pub struct LlmChunk {
    pub delta: ChunkDelta,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone)]
pub enum ChunkDelta {
    Text(String),
    ToolCall { id: String, name: String, delta: String },
    Thinking(String),
}

#[derive(Debug, Clone)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}
```

#### claude/config.rs

```rust
/// Claude Provider 配置
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    pub auth: ClaudeAuth,
    pub model: String,
    pub extra_headers: HashMap<String, String>,
}

/// Claude 认证方式
#[derive(Debug, Clone)]
pub enum ClaudeAuth {
    /// API Key 认证（官方或转发站）
    ApiKey(ApiKeyConfig),
    /// OAuth 认证
    OAuth { token_path: PathBuf },
}

impl ClaudeConfig {
    /// 从 API Key 创建（官方或转发站）
    pub fn with_api_key(key: String, base_url: Option<String>, model: String) -> Self {
        Self {
            auth: ClaudeAuth::ApiKey(ApiKeyConfig {
                key,
                base_url,
                provider: ApiKeyProvider::Anthropic,
            }),
            model,
            extra_headers: HashMap::new(),
        }
    }

    /// 从 OAuth Token 文件创建
    pub fn with_oauth(token_path: PathBuf, model: String) -> Self {
        Self {
            auth: ClaudeAuth::OAuth { token_path },
            model,
            extra_headers: HashMap::new(),
        }
    }
}
```

#### claude/provider.rs

```rust
pub struct ClaudeProvider {
    config: ClaudeConfig,
    compiler: ClaudeCompiler,
    oauth: Option<ClaudeOAuth>,
    http: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(config: ClaudeConfig) -> Self { ... }

    fn base_url(&self) -> &str {
        match &self.config.auth {
            ClaudeAuth::ApiKey(cfg) => {
                cfg.base_url.as_deref()
                    .unwrap_or("https://api.anthropic.com")
            }
            ClaudeAuth::OAuth { .. } => "https://api.anthropic.com",
        }
    }

    pub fn is_official(&self) -> bool {
        match &self.config.auth {
            ClaudeAuth::ApiKey(cfg) => cfg.base_url.is_none(),
            ClaudeAuth::OAuth { .. } => true,
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    fn id(&self) -> &str { "claude" }
    fn supported_models(&self) -> &[&str] {
        &["claude-sonnet-4-5-20250929", "claude-opus-4-5-20251101", "claude-3-5-haiku-20241022"]
    }
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        self.compiler.compile(primitive)
    }
    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse> { ... }
    fn needs_refresh(&self) -> bool {
        self.oauth.as_ref().map_or(false, |o| o.needs_refresh())
    }
    async fn refresh_auth(&mut self) -> crate::Result<()> {
        if let Some(oauth) = &mut self.oauth {
            oauth.refresh_token().await?;
        }
        Ok(())
    }
}
```

---

## 4. 分层容错机制

### 4.1 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        调用方（认知层）                           │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Gateway 层 (gateway.rs)                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Token Bucket  │  │  通用重试策略   │  │ Fallback Router │  │
│  │   (全局限流)    │  │ (429/5xx 重试)  │  │ (跨 Provider)   │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Provider 层 (provider/*.rs)                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  多端点 fallback │ Provider 特定错误检测 │ Token 自动刷新   ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 容错职责划分

| 层 | 容错类型 | 说明 |
|---|---------|------|
| **Provider** | 多端点 fallback | Antigravity: daily → sandbox |
| **Provider** | 特定错误检测 | Antigravity: "no capacity" |
| **Provider** | 认证状态维护 | IFlow: Cookie 保活 |
| **Provider** | Token 自动刷新 | OAuth providers |
| **Gateway** | 全局令牌桶限流 | 防止 API 雪崩 |
| **Gateway** | 通用错误重试 | 429/5xx 自动重试 |
| **Gateway** | 跨 Provider 降级 | Anthropic → OpenAI → Ollama |
| **Gateway** | 请求超时控制 | 统一超时配置 |

### 4.3 错误信号传递

Provider 层的错误需要携带足够的信息供 Gateway 层决策：

```rust
/// Provider 执行错误，带有重试信号
pub struct ProviderError {
    pub message: String,
    /// 是否应该在同一 Provider 重试 (例如 500, 429)
    pub retryable: bool,
    /// 是否应该触发跨 Provider 降级
    pub should_fallback: bool,
    /// 建议的重试延迟（毫秒）
    pub retry_after_ms: Option<u64>,
}
```

#### HTTP Client 连接池与资源控制
**架构红线**：Provider 层 (`auth/providers/*` 和 `provider/*`) **严禁**自行在内部调用 `reqwest::Client::builder().build()` 去重复创建 HTTP 连接池。
1. `reqwest::Client` 内部自带连接池 (Connection Pooling) 机制。如果在每次 `Provider::new()` 或者验证 Auth 时重新初始化 Client，会导致严重的文件描述符（SOCKET/TCP/TLS 握手）泄漏并拖垮网关。
2. 所有的 `Provider` 实例化时，都必须接受由 `Gateway` 或外部上下文注入的同一个 `reqwest::Client` 引用/克隆传递。
3. `Gateway` 负责持有全局唯一的 `Client`，统一定义 DNS 解析策略、Proxy 设置、超时阈值等。

### 4.4 错误处理流程

```
请求失败
    │
    ├─→ Provider 内部处理
    │       │
    │       ├─→ 多端点 fallback ─→ 成功 ─→ 返回结果
    │       │
    │       └─→ 内部重试耗尽 ─→ 返回带信号的错误
    │
    └─→ Gateway 层处理
            │
            ├─→ 通用重试 ─→ 成功 ─→ 返回结果
            │
            └─→ 触发降级 ─→ 切换 Provider ─→ 重试
```

---

## 5. 黑魔法代理层

### 5.1 多形态反代模型

围绕四个上游项目（CLIProxyAPI / newapi / ccswitch / Claude Code Router），统一整理多形态反代输出：

| 上游项目 | 在社区中常见角色 | 在本项目抽象的形态 |
|---|---|---|
| CLIProxyAPI | 本地 CLI 能力 API 化 | `Api` + `Cli` |
| newapi | 多渠道网关/聚合中转 | `Api` + `Auth` |
| ccswitch | Claude Code 请求切换层 | `Api` + `WebSocket` |
| Claude Code Router | Claude Code 路由分流 | `Api` + `WebSocket` |

### 5.2 核心类型

```rust
/// 反代接口形态
pub enum ProxyExposureKind {
    Api,        // HTTP JSON
    Auth,       // 鉴权代理
    WebSocket,  // 实时双工
    Cli,        // 本地进程桥接
}

/// 反代端点描述
pub struct ProxyExposure {
    pub kind: ProxyExposureKind,
    pub path: String,
    pub method: String,
    pub auth_header: Option<String>,
    pub auth_prefix: Option<String>,
    pub cli_command: Option<String>,
    pub cli_args: Vec<String>,
    pub notes: String,
}

/// 反代项目规格
pub struct BlackMagicProxySpec {
    pub target: BlackMagicProxyTarget,
    pub default_base_url: String,
    pub exposures: Vec<ProxyExposure>,
    pub notes: String,
}
```

### 5.3 统一调用准备器

`BlackMagicProxyClient::prepare_call(kind, request)` 根据形态生成标准化调用参数：

- `ProxyPreparedCall::Http(ProxyPreparedHttpCall)` — HTTP method/url/headers/body
- `ProxyPreparedCall::WebSocket(ProxyPreparedWsCall)` — ws url + 握手 header + init payload
- `ProxyPreparedCall::Cli(ProxyPreparedCliCall)` — command/args/env/stdin payload

---

## 6. 配置示例

```yaml
# ========== Claude ==========
# 官方 Claude
- provider: claude
  api_key: "sk-ant-xxx"
  model: "claude-sonnet-4-5"

# Claude 转发站（OpenRouter）
- provider: claude
  api_key: "sk-or-xxx"
  base_url: "https://openrouter.ai/api/v1"
  model: "anthropic/claude-sonnet-4.5"

# Claude OAuth
- provider: claude
  oauth_token: "~/.claude/user@example.com.json"
  model: "claude-sonnet-4-5"

# ========== OpenAI ==========
# 官方 OpenAI
- provider: openai
  api_key: "sk-xxx"
  model: "gpt-4o"

# OpenAI 转发站（Kimi）
- provider: openai
  api_key: "sk-xxx"
  base_url: "https://api.moonshot.cn/v1"
  model: "moonshot-v1-8k"

# ========== Gemini ==========
# 官方 Gemini AI Studio
- provider: gemini
  api_key: "AIzaSyxxx"
  model: "gemini-2.5-flash"

# Gemini 转发站
- provider: gemini
  api_key: "vk-xxx"
  base_url: "https://zenmux.ai/api"
  model: "gemini-2.5-pro"

# Vertex AI (Service Account)
- provider: gemini
  service_account: "/path/to/sa.json"
  model: "gemini-2.5-flash"
  location: "us-central1"

# Gemini CLI (OAuth)
- provider: gemini
  oauth_token: "~/.gemini-cli/user@gmail.com-project.json"
  model: "gemini-2.5-pro"

# ========== Codex ==========
- provider: codex
  api_key: "sk-xxx"
  model: "gpt-5-codex"

# ========== iFlow ==========
- provider: iflow
  cookie: "BXAuth=xxx;"
  model: "qwen3-max"
```

---

## 7. 设计原则总结

本文档定义的八大核心设计原则：

| 序号 | 原则 | 说明 |
|------|------|------|
| 1 | **认证与协议正交** | 认证方式和请求协议是两个独立的维度 |
| 2 | **协议优先** | Provider 按协议划分，不是按官方/转发站划分 |
| 3 | **配置区分类型** | 官方/转发站由 `base_url` 决定，不是类型差异 |
| 4 | **OAuth 独立实现** | 各 Provider 的 OAuth 差异太大，必须独立实现 |
| 5 | **原语中间层** | `PrimitiveRequest` 作为统一中间表示，解耦转换 |
| 6 | **统一 Trait 接口** | `LlmProvider` trait 提供统一调用接口 |
| 7 | **分层容错** | Provider 层处理特有容错，Gateway 层处理通用容错 |
| 8 | **错误信号传递** | `ProviderError` 携带重试/降级信号供 Gateway 决策 |

---

## 8. Examples 集成测试与范例规范

为了保证大模型客户端在独立环境下的可用性，以及提供快速试错的沙箱机制，`nl_llm` 采用 `examples/` 目录存放各 Provider 的独立端到端连通性测试。设计规范如下：

### 8.1 目录划分
每个 Provider 必须在 `examples/` 目录下建立自己专属的作用域，按核心功能分拆为子模块包：
```text
examples/
├── [provider_name]/
│   ├── auth/              # 身份验证/凭证流获取范例
│   │   ├── main.rs
│   │   └── [provider]_auth.bat
│   ├── chat/              # 标准生成/流式生成范例
│   │   ├── main.rs
│   │   ├── [provider]_chat.bat          # 测试全量生成
│   │   └── [provider]_chat_stream.bat   # 测试 sse 流式回显
│   └── models/            # 动态网关模型列表抓取范例（平台支持时）
│       ├── main.rs
│       └── [provider]_models.bat
```

### 8.2 main.rs 编写规范
1. **脱离主应用依赖**：`examples` 必须只依赖当前 `nl_llm` 包以及通用的三方库（如 `tokio`, `serde_json`）。绝对不允许反向依赖外部或上层 Workspace 的任何业务逻辑代码。
2. **凭据读取优先级**：优先从 `std::env::var` 读取密钥 / 环境变量（如 `GEMINI_API_KEY`、`IFLOW_COOKIE`）。对于复杂的授权模式（如 OAuth），需直接在内部完成 `TokenStorage::from_file()` 的装载机制探测。
3. **输出清晰**：所有的通信异常必须打印完整的 HTTP 状态码与原始服务端文字 `resp.text().await`，以便快速定位问题来源。

### 8.3 .bat 端到端测试脚本规范
为了让全部协同开发者或 AI 克隆项目后能获得“开箱即用”的验证体验，每个 `main.rs` 旁必须配有 `.bat` 启动脚本。极简规范如下：

1. **统一的控制台抬头**：脚本运行必须打印明显的分割线和当前测试的模块名，如 `echo === Gemini Chat Test ===`。
2. **终端乱码与注入防护**：
   - 必须确保 `.bat` 文件**没有任何 UTF-8 BOM 隐形头**，以防止 `cmd.exe` 在解析第一行路径时将其截断报错（直接抛出 `:\Users...` 这种毁容级的路径错误）。
   - 设置终端代码页 `chcp 65001 >nul`，确保输出 Emoji 和宽体字符时不出现问号乱码。
3. **安全的兜底降权密钥**：
   - 测试脚本必须首选宿主机的全局环境参数中的 KEY。
   - 如果用户未配置，**允许在脚本内部硬编码提供一张 fallback 的兜底测试 Key**。但是一旦动用兜底 Key，必须打印黄色的 `[WARNING] Using default embedded API_KEY...` 提示用户，严防机密被长久泄漏滥用。
4. **包装执行接口**：将繁复的 `cargo run --example [模块路径] -- [参数]` 彻底隐藏包装进 `.bat` 内部，仅把最高频最口语的参数（例如提词 `prompt`）留作外传参数 `$1`。

综上所述，`examples/` 已经不再是单纯给新人看的“使用演示 (Demonstration)”，它现在直接充当工程的 **“端到端集成测试” (Integration E2E Tests)** 前沿阵地。每个 Provider 合入主分支前，必须保证其范例结构与其 `.bat` 能够顺畅跑通。

---

*本文档作为 `nl_llm` 模块的完整设计规范，指导后续实现开发。*
