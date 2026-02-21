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
│   │   ├── cli.rs                # OAuth 认证
│   │   └── antigravity.rs        # OAuth 认证
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
    IFlow,
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
    /// 是否应该在同一 Provider 重试
    pub retryable: bool,
    /// 是否应该触发跨 Provider 降级
    pub should_fallback: bool,
    /// 建议的重试延迟（毫秒）
    pub retry_after_ms: Option<u64>,
}
```

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

*本文档作为 `nl_llm` 模块的完整设计规范，指导后续实现开发。*
