# 分层容错机制设计

## 概述

NeuroLoom 的 `nl_llm` 模块采用分层容错架构，将容错职责清晰地划分到不同层次，实现关注点分离和代码复用。

## 架构图

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
│  │                    Antigravity Provider                      ││
│  │  ┌─────────────────┐  ┌─────────────────────────────────┐   ││
│  │  │ 多 Base URL     │  │ Provider 特定错误重试           │   ││
│  │  │ Fallback        │  │ ("no capacity" / 特定错误码)    │   ││
│  │  │ (daily→sandbox) │  │                                 │   ││
│  │  └─────────────────┘  └─────────────────────────────────┘   ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                      IFlow Provider                          ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │ Cookie 保活 / API Key 自动刷新                          │││
│  │  └─────────────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                   其他 Provider...                          ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## 容错职责划分

### Provider 层容错

**职责**：处理 Provider 特有的容错逻辑

| 容错类型 | 示例 | 实现位置 |
|---------|------|---------|
| 多端点 fallback | Antigravity: daily → sandbox | `provider/antigravity.rs` |
| Provider 特定错误检测 | Antigravity: "no capacity" | `provider/antigravity.rs` |
| 认证状态维护 | IFlow: Cookie 保活 | `provider/iflow.rs` |
| Token 自动刷新 | OAuth providers | 各 provider 内部 |

**设计原则**：
- Provider 内部实现细节不暴露给上层
- 每个 Provider 可以有自己独特的容错策略
- Provider 完成内部容错后，仍失败的错误向上传递

**示例代码**：

```rust
// provider/antigravity.rs
pub async fn complete(&mut self, ast: &PromptAst) -> crate::Result<String> {
    for retry_attempt in 0..NO_CAPACITY_MAX_RETRIES {
        // 遍历所有 Base URL
        loop {
            let base_url = self.current_base_url();
            // ... 发送请求 ...

            // Provider 特定错误检测
            if Self::is_no_capacity_error(status, &raw_text) {
                if self.try_next_base_url() {
                    continue; // 尝试下一个 Base URL
                }
                // 所有 URL 都失败了，延迟后重试
                break;
            }

            // 429 尝试下一个 Base URL
            if status.as_u16() == 429 {
                if self.try_next_base_url() {
                    continue;
                }
                // 所有 URL 都被限流，返回错误让 Gateway 处理
                return Err(...);
            }
        }
    }
}
```

### Gateway 层容错

**职责**：处理所有 Provider 共享的通用容错逻辑

| 容错类型 | 说明 | 实现位置 |
|---------|------|---------|
| 全局令牌桶限流 | 防止 API 雪崩 | `token_bucket.rs` |
| 通用错误重试 | 429/5xx 自动重试 | `gateway.rs` (待实现) |
| 跨 Provider 降级 | Anthropic → OpenAI → Ollama | `fallback.rs` |
| 请求超时控制 | 统一超时配置 | `gateway.rs` |

**设计原则**：
- 不关心 Provider 内部实现细节
- 只根据错误类型和状态码做决策
- 提供统一的容错接口给上层调用

**示例代码**：

```rust
// gateway.rs (扩展后)
impl LlmGateway {
    pub async fn execute_with_fallback(&self, ast: &PromptAst) -> crate::Result<String> {
        self.token_bucket.acquire().await?;

        for attempt in 0..self.max_retries {
            let provider = self.fallback.current()?;

            match self.execute_provider(provider, ast).await {
                Ok(result) => return Ok(result),
                Err(e) if self.should_retry(&e) => {
                    // 通用重试逻辑
                    self.delay_before_retry(attempt).await;
                    continue;
                }
                Err(e) if self.should_fallback(&e) => {
                    // 切换到下一个 Provider
                    self.fallback.fallback();
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

## 错误信号传递

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

## 错误处理流程

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

## 实现清单

### 已完成
- [x] Provider 层：Antigravity 多 Base URL fallback
- [x] Provider 层：Antigravity "no capacity" 检测
- [x] Provider 层：Token 刷新提前量优化（50 分钟）
- [x] Gateway 层：Token Bucket 全局限流
- [x] Gateway 层：FallbackRouter 跨 Provider 降级框架

### 待实现
- [ ] Gateway 层：通用 429/5xx 重试逻辑
- [ ] Gateway 层：Retry-After 头解析
- [ ] Gateway 层：统一的 ProviderError 类型
- [ ] Gateway 层：execute_with_fallback 方法

## 参考资料

- CLIProxyAPI `antigravity_executor.go`：Provider 层容错实现
- 架构文档 `docs/架构.md`：整体设计哲学
