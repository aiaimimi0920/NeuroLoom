# nl_llm 黑魔法代理接口梳理（四项目反代方式落地）

## 你要的目标（本次已实现）

围绕四个上游项目（CLIProxyAPI / newapi / ccswitch / Claude Code Router），不是只看“一个 API 入口”，而是把它们常见的**多形态反代输出**统一整理并在 `nl_llm` 里实现：

- API 反代（HTTP JSON）
- Auth 反代（鉴权代理）
- WebSocket 反代（实时双工）
- CLI 反代（本地进程桥接）

实现文件：`crates/nl_llm/src/provider/black_magic_proxy.rs`

---

## 四项目反代方式整理（统一视图）

| 上游项目 | 在社区中常见角色 | 在本项目抽象的形态 |
|---|---|---|
| CLIProxyAPI | 本地 CLI 能力 API 化 | `Api` + `Cli` |
| newapi | 多渠道网关/聚合中转 | `Api` + `Auth` |
| ccswitch | Claude Code 请求切换层 | `Api` + `WebSocket` |
| Claude Code Router | Claude Code 路由分流 | `Api` + `WebSocket` |

> 说明：这里不是复刻各项目全部细节，而是抽取“能稳定接入 NeuroLoom 的反代接口能力”。

---

## 代码中如何实现

### 1) 多反代形态模型

新增：
- `ProxyExposureKind`：`Api / Auth / WebSocket / Cli`
- `ProxyExposure`：描述 path/method/auth header/cli command 等
- `BlackMagicProxySpec`：一个项目可挂多个 `exposures`

### 2) 统一调用准备器

`BlackMagicProxyClient::prepare_call(kind, request)` 会根据 kind 生成：

- `ProxyPreparedCall::Http(ProxyPreparedHttpCall)`
- `ProxyPreparedCall::WebSocket(ProxyPreparedWsCall)`
- `ProxyPreparedCall::Cli(ProxyPreparedCliCall)`

这样上层只要选“目标项目 + 反代形态”，就能拿到可执行的标准化调用参数。

### 3) 四项目 profile 已内置

`BlackMagicProxyCatalog::all_specs()` 里已内置四项目默认 profile（默认 base_url + 暴露形态 + 鉴权规则 + 路径）。

---

## 现在能直接做什么

1. **做 API 网关调用**：拿到标准 HTTP method/url/headers/body。  
2. **做鉴权代理切换**：把 credential 统一注入到约定 header。  
3. **做 WS 会话接入**：拿到 ws/wss url + 握手 header + init payload。  
4. **做 CLI 桥接执行**：拿到 command/args/env/stdin payload。  

---

## 下一步建议

- 在 `nl_llm` 增加真实执行器：
  - HTTP 执行器（reqwest）
  - WS 执行器（tokio-tungstenite）
  - CLI 执行器（复用 `cli_proxy` / PTY）
- 将执行失败接到 `fallback`（按 target+kind 做回退）。
- 增加每种反代形态的契约测试（本地 mock server + mock ws）。

