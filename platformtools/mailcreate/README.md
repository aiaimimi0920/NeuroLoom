# platformtools/mailcreate

目标：把“临时邮箱能力”抽象成一个可独立存在的 Platform Tool：

- `server/`：可部署的服务端源码（Cloudflare Worker + D1 + Email Routing）
- `client/`：统一客户端 SDK（供多个自动化注册器复用）

## Q1：服务是否已经部署到 Cloudflare？本地是否还有代码？

- 已部署：Cloudflare 上存在 Worker 脚本 `cloudflare_temp_email`，并绑定自定义域名 `mail.aiaimimi.com`。
- 本地仍保留源码：用于后续修改/再部署。

## 当前环境关键配置
- Worker 自定义域名：`mail.aiaimimi.com`
- 收件域名（Email Routing）：`aiaimimi.com`

服务端配置文件示例：[`wrangler.toml`](../mailcreate/server/cloudflare_temp_email/worker/wrangler.toml:1)
