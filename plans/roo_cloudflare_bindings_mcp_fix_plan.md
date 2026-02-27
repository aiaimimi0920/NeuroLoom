# Roo VSCode 扩展连接 Cloudflare Workers Bindings MCP 排障与修复计划

> 目标：让 Roo 通过 MCP 连接 Cloudflare 官方 Workers Bindings 远程 MCP Server，并可稳定复现。

## 1. 目标端点确认
Cloudflare 官方文档明确：Bindings MCP 支持 `streamable-http` 传输，端点是：
- `https://bindings.mcp.cloudflare.com/mcp`

来源：[`plans/roo_cloudflare_bindings_mcp_fix_plan.md:8`](plans/roo_cloudflare_bindings_mcp_fix_plan.md:8)（引用内容来自 Cloudflare Docs：Cloudflare's own MCP servers）

## 2. 你当前配置的问题点（已定位）
你现在的 Roo MCP 配置里 cloudflare 是：
- `command: npx`
- `args: [ mcp-remote, https://bindings.mcp.cloudflare.com/sse ]`
见：[`../../../../../vmjcv/AppData/Roaming/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json:256`](../../../../../vmjcv/AppData/Roaming/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json:256)

潜在问题（优先级从高到低）：
1) **未加 `-y`**：Windows 上 `npx` 首次安装 `mcp-remote` 可能等待交互确认导致“卡住”。
2) **使用了 `/sse`（deprecated）而非 `/mcp`**：Cloudflare 官方推荐 `/mcp`；在代理环境下更易成功。
3) **Clash 代理未显式注入到 `mcp-remote`**：`mcp-remote` 只有在加 `--enable-proxy` 且提供 `HTTP_PROXY/HTTPS_PROXY` 时才会走系统代理环境变量（不同客户端/运行环境下系统代理不一定生效）。
4) **OAuth 本地缓存污染**：`mcp-remote` 会把 OAuth 状态存在 `~/.mcp-auth`，旧状态常导致 `invalid_grant`/401/回调失败。

## 3. 建议的 Roo 配置（可直接替换 cloudflare 节点）
把 [`../../../../../vmjcv/AppData/Roaming/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json:256`](../../../../../vmjcv/AppData/Roaming/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json:256) 这一段替换为（注意：不要在文件里写任何 Cloudflare Token/Key）：

```json
{
  "cloudflare": {
    "command": "npx",
    "args": [
      "-y",
      "mcp-remote@latest",
      "https://bindings.mcp.cloudflare.com/mcp",
      "--transport",
      "http-only",
      "--enable-proxy",
      "--debug"
    ],
    "env": {
      "HTTP_PROXY": "http://127.0.0.1:42344",
      "HTTPS_PROXY": "http://127.0.0.1:42344",
      "NO_PROXY": "localhost,127.0.0.1"
    }
  }
}
```

说明：
- `http-only`：强制走 HTTP（streamable-http），避免 SSE 兼容性问题。
- `--debug`：把详细日志写入 `~/.mcp-auth/*_debug.log`，便于定位 OAuth / 回调 / 代理问题。
- 代理端口 `42344`：来自 FlClash 当前 `mixed-port`（见 [`../../../../../vmjcv/AppData/Roaming/com.follow/clash/config.yaml:5`](../../../../../vmjcv/AppData/Roaming/com.follow/clash/config.yaml:5)）。如果你的系统代理实际用的是 7890/7891，需要把这里改成真实端口。

## 4. 清理 OAuth 缓存（必做）
在 Windows 上删除目录：
- `%USERPROFILE%\\.mcp-auth`

目的：清理旧 token / code_verifier / session，避免 `invalid_grant`。

## 5. 终端最小化验证（先于 Roo）
优先用命令行验证远端可连、OAuth 流可完成、tools/list 可用，再回到 Roo。

建议用（PowerShell 或 CMD 均可，示例以 cmd 思路写）：
- 运行 `npx -y mcp-remote@latest https://bindings.mcp.cloudflare.com/mcp --transport http-only --enable-proxy --debug`
- 观察是否自动打开浏览器进行 Cloudflare OAuth
- 成功后应能完成 tools/list（或至少连接完成且无报错）

> 如果命令行都失败，Roo 里也必然失败；此时直接看 `~/.mcp-auth/*_debug.log`（脱敏后）定位根因。

## 6. 常见失败模式 -> 对应处理
- 卡在 installing packages：加 `-y` + 固定 `mcp-remote@latest`
- TLS/证书报错（SELF_SIGNED_CERT / UNABLE_TO_VERIFY_LEAF_SIGNATURE）：代理在做 MITM，需要配置 `NODE_EXTRA_CA_CERTS`（谨慎）
- OAuth 回调失败：检查本地回调端口是否被占用；可在 args 里给 `mcp-remote` 追加自定义端口（URL 后追加一个端口号）
- 代理不生效：确认 Clash 实际监听端口（mixed-port vs 7890/7891），并确保 `HTTP_PROXY/HTTPS_PROXY` 指向正确端口

## 7. 验收标准
- Roo 中 cloudflare MCP server 显示连接成功
- 可列出 tools（tools/list）
- 触发一次真实工具调用并返回有效结果

