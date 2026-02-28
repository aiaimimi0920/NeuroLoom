# DNSHE → Cloudflare 权威 DNS 委派（用于 Email Routing 收件）

目的：让 `*.cc.cd` 这类 DNSHE 免费域名能够 **通过 Cloudflare Email Routing 收件**，从而给 MailCreate 提供一个“多域名收件池”，降低单域名被封禁的风险。

关键点：
- Cloudflare Email Routing 需要 Cloudflare 作为权威 DNS（也就是要把该域名的 DNS 委派给 Cloudflare）。
- 在 DNSHE 的 UI 里可以添加 **NS 记录（子域授权 / DNS 服务器）**，把域名委派给 Cloudflare。

## 你要达到的效果
- API（MailCreate Worker）仍然固定一个地址，例如 `MAILCREATE_BASE_URL=https://mail.example.com`。
- 但从 API 创建到的邮箱地址会在多个域名中随机分配（服务端逻辑会随机挑选 `DOMAINS` 里的域名）。

服务端随机选域名逻辑在：[`newAddress()`](platformtools/mailcreate/server/cloudflare_temp_email/worker/src/common.ts:205)

## 自动化边界（重要）
当前 DNSHE 的公开 API（`dns_records`）在创建 `NS` 类型记录时会返回类似 `{"error":"invalid type"}` 的错误；因此 **“写 NS 完整委派”无法 100% 端到端自动化**。

本仓库脚本可以做到：
1) 自动在 Cloudflare 创建/查找 zone，并拿到 Cloudflare 分配的 nameserver；
2) 输出每个域名 → NS 的映射；
3) 你仍需在 DNSHE 控制台 **手工** 把 NS 填进去完成委派。

## 步骤 1：批量在 Cloudflare 创建 Zone 并导出 NS（脚本）
脚本：[`bootstrap_cf_zones_and_delegate.py`](platformtools/dnshe/bootstrap_cf_zones_and_delegate.py:1)

1) 准备本机 secrets 文件（会被 gitignore）：
- 复制模板：[`platformtools/dnshe/.dev.vars.example`](platformtools/dnshe/.dev.vars.example:1)
- 填写为：`platformtools/dnshe/.dev.vars`

2) 执行（推荐先 dry-run；并用 `--skip-dnshe` 只做 Cloudflare 侧动作 + 输出 NS 映射）：
```powershell
python platformtools\dnshe\bootstrap_cf_zones_and_delegate.py `
  --domains artai.cc.cd,artllm.cc.cd `
  --dry-run `
  --skip-dnshe

python platformtools\dnshe\bootstrap_cf_zones_and_delegate.py `
  --domains artai.cc.cd,artllm.cc.cd `
  --skip-dnshe
```

运行后会输出 `NS_MAP_JSON={...}`，里面是每个域名对应的两条 Cloudflare nameserver。

## 步骤 2：在 DNSHE 手工委派（设置 NS）
DNSHE 后台 → 对应域名 → 添加解析记录（通常需要添加两条）：
- 类型：NS记录（DNS服务器/子域授权）
- 记录名：`@`（apex）
- 内容：填写 Cloudflare 的 NS（两条分别添加）

> 建议：委派后不要再在 DNSHE 同时维护 A/MX/TXT 等 apex 记录，避免解析混乱。

## 步骤 3：验证 NS 已生效
在 Windows 上可以用 `nslookup` 验证（可能需要等待 DNS 缓存/传播）：
```cmd
nslookup -type=ns artai.cc.cd
```

看到返回的 NS 与 Cloudflare 分配的一致即可。

## 步骤 4：在 Cloudflare 启用 Email Routing 并配置 Catch-all → Worker
对每个域名在 Cloudflare 里：
1. Email → Email Routing → Enable
2. Routes / Routing rules：设置 Catch-all 规则，action 选择 **send to Worker**（Email Worker）

参考仓库文档：[`email-routing.md`](platformtools/mailcreate/server/cloudflare_temp_email/vitepress-docs/docs/zh/guide/email-routing.md:1)

## 步骤 5：把 MailCreate 的 `DOMAINS` 配成域名池
MailCreate Worker 需要知道有哪些域名可用：
- 在 Worker 环境变量里配置 `DOMAINS` 为 JSON 数组
- 同时设置 `DEFAULT_DOMAINS`

本地示例（仅供参考）：[`wrangler.toml`](platformtools/mailcreate/server/cloudflare_temp_email/worker/wrangler.toml:23)

## 步骤 6：自动注册器不强制单域名
自动注册器已改为：默认不设置 `MAILCREATE_DOMAIN`，让服务端随机选域名。
见：[`create_temp_mailbox()`](platformtools/auto_register/oai_register_cn/main.py:120)

---

## 附：DNSHE API 写 NS 的脚本（目前可能失败）
脚本：[`delegate_to_cloudflare.py`](platformtools/dnshe/delegate_to_cloudflare.py:1)

说明：该脚本尝试通过 DNSHE API 的 `dns_records.create` 写入 `NS` 记录；但如果 DNSHE API 继续拒绝 `NS` 类型（`invalid type`），则只能采用“步骤 2”的 UI 手工方式完成委派。
