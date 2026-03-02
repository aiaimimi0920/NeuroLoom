# InfiniteRefill v2 运维与对接指令

## 1) 普通用户客户端是否需要更新

需要。普通续杯已从旧的 `auth_json` 直返改为短时下载链接模式：

- 服务端接口：`POST /v1/refill/topup`
- 返回：`accounts[].download_url`
- 客户端需按链接下载 JSON 后再写入本地账号池

已完成改造位置：

- `platformtools/auto_register/codex_register/main.py`
- `platformtools/auto_register/codex_register/browser_version/main.py`

## 2) 生成用户（手动分发）接口

接口：`POST /admin/packages/issue`

鉴权：

- `Authorization: Bearer <ADMIN_TOKEN>`
- 若配置了管理员二次校验：`X-Admin-Guard: <ADMIN_GUARD>`

推荐请求体（type=user）：

```json
{
  "type": "user",
  "count": 10,
  "label": "manual-batch-20260302",
  "server_url": "https://refill.aiaimimi.com",
  "max_accounts_per_user": 20,
  "min_accounts_required": 5,
  "ttl_minutes": 60
}
```

说明：

- `max_accounts_per_user`：单用户最大持有上限（也是每包目标分配量）
- `min_accounts_required`：单用户最小分配门槛；不足则该用户不会入库，且本批次停止继续生成
- 返回中包含：
  - `packages[].download_url`
  - `manifest.download_url`
  - `manifest.mapping`（`{"数字": "用户key"}`）

## 3) 查询服务端统计信息接口

接口：`GET /admin/stats`

当前重点字段：

- `accounts_v2_public`：可分配公有池数量
- `accounts_v2_total`
- `users_v2_total`
- `user_keys_v2_total`
- `accounts_legacy_total`

## 4) 可调参数（通过指令）

### 4.1 手动分发

通过 `POST /admin/packages/issue` 请求体调节：

- `count`
- `max_accounts_per_user`
- `min_accounts_required`
- `ttl_minutes`

### 4.2 普通续杯目标池

客户端环境变量：

- `TARGET_POOL_SIZE`

### 4.3 缺一补一策略

已启用：当 `pool_size < TARGET_POOL_SIZE` 即触发续杯。

## 5) 数据库清空（用于统一重传）

已按你的要求执行 v2/legacy 相关业务数据清空（保留必要系统表结构与基础 key 表）。

可复核：

- `accounts_legacy=0`
- `accounts_v2=0`
- `users_v2=0`
- `user_keys_v2=0`
- `probes=0`
- `invalid_accounts=0`
- `exhausted_accounts=0`
- `package_batches=0`

## 6) 回滚方案（最小）

- 代码回滚：重新部署上一版本 Worker。
- 数据回滚：若有备份，可通过管理接口导入；无备份则重新上传账号数据。

> 建议：每次执行批量清库前，先导出一次备份。