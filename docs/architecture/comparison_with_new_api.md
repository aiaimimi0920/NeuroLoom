# NeuroLoom vs New-API 架构对比分析

## ���述

本文档对比分析 NeuroLoom 的 LLM 集成架构与 new-api 的渠道管理架构，评估 NeuroLoom 是否能完全覆盖 new-api 的渠道添加功能。

## 1. 核心架构对比

### new-api 架构（网关模式）

```
┌─────────────────────────────────────────────────────────────┐
│                        API Gateway                           │
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │   Channel    │───▶│   Selector   │───▶│   Adaptor    │   │
│  │    Pool      │    │ (weighted)   │    │              │   │
│  └──────────────┘    └──────────────┘    └──────────────┘   │
│         │                   │                    │           │
│    [Multi-Key]        [Priority/Retry]     [Protocol]       │
│    [Status]           [Load Balance]       [Convert]        │
│    [Balance]                                                   │
└─────────────────────────────────────────────────────────────┘
```

**关键特点：**
- Channel 模型包含完整的渠道配置
- Ability 表实现 (group, model) → channel_id 映射
- 多 Key 支持（轮询/随机模式）
- 加权随机选择 + 优先级回退
- 响应时间统计 + 自动禁用

### NeuroLoom 架构（客户端模式）

```
┌─────────────────────────────────────────────────────────────┐
│                        LlmClient                             │
│                                                              │
│  ┌─────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐  │
│  │  Site   │ → │   Auth   │ → │ Protocol │ → │  Model   │  │
│  └─────────┘   └──────────┘   └──────────┘   └──────────┘  │
│       │              │              │              │         │
│  [BaseURL]     [Credential]    [Format]      [Resolve]      │
│  [Headers]     [Refresh]       [Hook]        [Capability]   │
└────────────────────────��────────────────────────────────────┘
```

**关键特点：**
- 四维正交分解：Site, Auth, Protocol, Model
- Pipeline 处理流程
- Preset 开箱即用
- Protocol Hook 平台扩展

## 2. 功能映射表

### 2.1 完全覆盖的功能

| new-api 特性 | 实现位置 | NeuroLoom 对应 | 说明 |
|-------------|---------|---------------|------|
| Channel.Type → APIType → Adaptor | constant/channel.go, relay/relay_adaptor.go | Preset → Protocol + Hook | 更灵活的组装方式 |
| Channel.BaseURL | model/channel.go | Site.base_url() | ✅ 完全对应 |
| Channel.Key | model/channel.go | Authenticator | ✅ 完全对应 |
| Channel.Models | model/channel.go | ModelResolver | ✅ 完全对应 |
| Channel.ModelMapping | model/channel.go | ModelResolver.resolve() | ✅ 别名映射 |
| Channel.Setting (JSON) | dto/channel_settings.go | PipelineContext + PrimitiveRequest.extra | ✅ 灵活扩展 |
| Channel.HeaderOverride | relay/channel/api_request.go | Site.extra_headers() + Hook.before_send() | ✅ 完全对应 |
| Channel.ParamOverride | model/channel.go | PrimitiveRequest.extra | ✅ 完全对应 |
| Channel.Other (regions等) | model/channel.go | Site/UrlContext | ✅ 上下文感知 |

### 2.2 协议适配对比

| new-api 实现 | NeuroLoom 实现 | 优势 |
|-------------|---------------|------|
| `ChannelType2APIType()` 两层映射 | Preset 直接组装 | NeuroLoom 更直观 |
| `GetAdaptor(apiType)` 工厂 | `LlmClient::from_preset()` | NeuroLoom 更易用 |
| 单一 Adaptor 接口 | Protocol + Hook 组合 | NeuroLoom 更灵活 |
| 硬编码的类型映射 | Preset Registry 动态注册 | NeuroLoom 更易扩展 |

### 2.3 平台特定处理对比

| 场景 | new-api 实现 | NeuroLoom 实现 |
|------|-------------|---------------|
| Vertex AI 多协议 | Adaptor 内部判断 model 前缀 | Gemini Protocol + ProtocolHook |
| iFlow Thinking 模型 | Adaptor 内部特殊处理 | IflowThinkingHook.after_pack() |
| OpenRouter provider 字段 | Adaptor 内部添加 | OpenRouterHook.after_pack() |
| Claude beta 查询 | ChannelOtherSettings | ClaudeExtension 或 Hook |

## 3. new-api 的"小巧思"分析

### 3.1 多 Key 支持 ⚠️ NeuroLoom 未实现

**new-api 实现：**
```go
type ChannelInfo struct {
    IsMultiKey             bool                  `json:"is_multi_key"`
    MultiKeySize           int                   `json:"multi_key_size"`
    MultiKeyStatusList     map[int]int           `json:"multi_key_status_list"`
    MultiKeyDisabledReason map[int]string        `json:"multi_key_disabled_reason"`
    MultiKeyDisabledTime   map[int]int64         `json:"multi_key_disabled_time"`
    MultiKeyPollingIndex   int                   `json:"multi_key_polling_index"`
    MultiKeyMode           constant.MultiKeyMode `json:"multi_key_mode"` // random or polling
}
```

**特点：**
- 支持 random（随机）和 polling（轮询）两种模式
- 每个 Key 独立状态管理
- 单个 Key 失效不影响其他 Key

**NeuroLoom 建议实现：**
```rust
pub struct MultiKeyAuthenticator {
    keys: Vec<Arc<dyn Authenticator>>,
    mode: MultiKeyMode,
    current_index: AtomicUsize,
    disabled: RwLock<HashSet<usize>>,
}

pub enum MultiKeyMode {
    Random,
    RoundRobin,
    Weighted(Vec<f32>),
}
```

### 3.2 渠道选择器 ⚠️ 网关功能，不在 SDK 范围

**new-api 实现：**
```go
// Ability 表：group + model → channel_id
type Ability struct {
    Group     string  `json:"group" gorm:"primaryKey"`
    Model     string  `json:"model" gorm:"primaryKey"`
    ChannelId int     `json:"channel_id" gorm:"primaryKey"`
    Enabled   bool    `json:"enabled"`
    Priority  *int64  `json:"priority"`
    Weight    uint    `json:"weight"`
}

// 选择算法
func SelectChannel(group, model string) *Channel {
    // 1. 查询 (group, model, enabled=true)
    // 2. 过滤 max priority
    // 3. 加权随机选择
    // 4. 支持 auto-group 跨组重试
}
```

**NeuroLoom 定位分析：**
- NeuroLoom 是**客户端 SDK**，不是网关
- 渠道选择、负载均衡属于**网关层功能**
- 但可以作为**独立模块**实现，供网关使用

**建议实现：**
```rust
// 作为独立模块
pub struct ChannelSelector {
    channels: Vec<LlmClient>,
    strategy: SelectionStrategy,
}

pub enum SelectionStrategy {
    RoundRobin,
    WeightedRandom(Vec<f32>),
    PriorityFirst,
    Fastest,
}

impl ChannelSelector {
    pub async fn complete(&mut self, req: &PrimitiveRequest) -> Result<LlmResponse> {
        let selected = self.select()?;
        selected.complete(req).await
    }
}
```

### 3.3 Coding Plan URL 映射 ✅ 可实现

**new-api 实现：**
```go
var ChannelSpecialBases = map[string]ChannelSpecialBase{
    "glm-coding-plan": {
        ClaudeBaseURL: "https://open.bigmodel.cn/api/anthropic",
        OpenAIBaseURL: "https://open.bigmodel.cn/api/coding/paas/v4",
    },
    "kimi-coding-plan": {
        ClaudeBaseURL: "https://api.kimi.com/coding",
        OpenAIBaseURL: "https://api.kimi.com/coding/v1",
    },
}
```

**NeuroLoom 可通过 Site 实现：**
```rust
pub struct CodingPlanSite {
    inner: DefaultSite,
    provider: CodingPlanProvider,
}

impl Site for CodingPlanSite {
    fn build_url(&self, ctx: &UrlContext) -> String {
        match ctx.protocol_hint {
            ProtocolHint::Claude => self.provider.claude_base_url(),
            ProtocolHint::OpenAI => self.provider.openai_base_url(),
        }
    }
}
```

### 3.4 Header Override 机制 ✅ 已支持

**new-api 实现：**
```go
// 规则：
// - "*": 透传所有请求头
// - "re:<regex>": 透传匹配的请求头
// - key-value: 显式覆盖
// 占位符：{api_key}, {client_header:<name>}
```

**NeuroLoom 已有对应能力：**
```rust
// Site.extra_headers() - 静态 headers
// ProtocolHook.before_send() - 动态 headers
// PipelineContext 中可访问原始请求
```

### 3.5 错误码映射 ⚠️ 可增强

**new-api 实现：**
```go
type Channel struct {
    StatusCodeMapping *string `json:"status_code_mapping"` // 错误码映射
}

// 示例：将 429 映射为其他错误
```

**NeuroLoom 建议：**
```rust
pub struct ErrorMapping {
    mappings: HashMap<u16, StandardError>,
}

impl ProtocolHook for ErrorMappingHook {
    fn after_receive(&self, ctx: &mut PipelineContext, resp: &mut Response) {
        if let Some(error) = self.mappings.get(&resp.status().as_u16()) {
            // 转换错误
        }
    }
}
```

## 4. 总结

### 4.1 覆盖度评估

| 类别 | 覆盖情况 |
|------|---------|
| 协议适配 | ✅ 100% |
| 认证方式 | ✅ 100% |
| 模型解析 | ✅ 100% |
| 平台扩展 | ✅ 100% |
| 多 Key 支持 | ⚠️ 0% (需补充) |
| 渠道选择 | ⚠️ 网关功能，非 SDK 范围 |

### 4.2 NeuroLoom 优势

1. **更清晰的架构**：四维正交分解 vs 单一 Channel 模型
2. **更灵活的组装**：Preset + Hook vs 硬编码类型映射
3. **更易扩展**：注册机制 vs 修改常量
4. **更好的复用**：Protocol 独立于平台

### 4.3 建议补充

1. **MultiKeyAuthenticator**：支持多 Key 轮询/随机
2. **ChannelSelector 模块**：作为独立组件供网关场景使用
3. **Metrics 收集**：响应时间、成功率统计
4. **ErrorMapping**：可配置的错误码映射

## 5. 结论

NeuroLoom 的架构设计**完全能够覆盖** new-api 的渠道添加核心功能，且在以下方面更具优势：
- 架构清晰度
- 扩展灵活性
- 平台复用性

需要补���的功能主要是**多 Key 支持**，这可以作为 Authenticator 的扩展实现。

至于**渠道选择、负载均衡、自动禁用**等网关层功能，属于不同层次的关注点，可以作为独立模块实现，不影响 NeuroLoom 作为客户端 SDK 的定位。
