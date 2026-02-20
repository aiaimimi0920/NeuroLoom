# nl_llm 实施清单（基于 docs/架构.md）

## 1. 模块灵魂定位

`nl_llm` 不只是“调用模型 API”，而是 **模型无关网关 + 防腐层 + 稳定性中枢**。

对应架构文档要点：
- Prompt AST 与方言编译（Anthropic XML / OpenAI JSON / Ollama ChatML）
- 前缀缓存（动静分离）
- 全局限流与反压
- 断流重试与降级
- CLI / 反代统一接入

## 2. 本次落地内容

### 2.1 Prompt AST 与方言编译
- 新增 `prompt_ast.rs`：`PromptAst` / `PromptNode`
- 支持编译输出：
  - `to_anthropic_xml()`
  - `to_openai_messages()`
  - `to_chatml()`

### 2.2 Provider 编译实现
- `openai.rs`：实现 `compile_request(&PromptAst)`
- `anthropic.rs`：实现 `compile_request(&PromptAst)`
- `ollama.rs`：实现 `compile_request(&PromptAst)`
- `complete()` 不再是固定 placeholder，而是返回“已编译请求”的可观测结果。

### 2.3 Gateway 编排层
- 新增 `gateway.rs`
- `LlmGateway::prepare_request()`：
  - 先走令牌桶限流
  - 读取 fallback 当前 provider
  - 调用对应 provider 编译 AST
  - 输出统一 `GatewayPreparedRequest`

## 3. 下一步（建议）

1. 将 `compile_request` 结果接入真实 HTTP/WS/CLI transport 执行器。
2. 将失败重试与 fallback 切换整合到统一 `execute_with_fallback()`。
3. 把 `PromptAst` 接到认知层，替代字符串拼接输入。
4. 增加端到端测试（mock provider + fallback 触发场景）。
