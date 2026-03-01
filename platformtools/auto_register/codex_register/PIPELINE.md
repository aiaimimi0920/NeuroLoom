# codex_register 预留目录的「流水线含义」对照 InfiniteRefill

> 目的：解释 `codex_auth / wait_update / need_fix_auth / fixed_success / fixed_fail` 这套目录的预期用法，并用 InfiniteRefill 的“Uploader/Repairer + 任务流”做类比。

## 1) InfiniteRefill 的关键思路（抽象出来的模式）

InfiniteRefill 的服务端/客户端把协作者分成三类角色，并围绕“提交/上报/领取任务/提交结果”来组织流转：

- 角色与权限（`USER_KEY / UPLOAD_KEY / ADMIN_TOKEN`）分层：见 [`platformtools/InfiniteRefill/server/README.md:3`](../../InfiniteRefill/server/README.md:3)、[`platformtools/InfiniteRefill/server/DESIGN.md:21`](../../InfiniteRefill/server/DESIGN.md:21)
- Uploader（热心群众）：负责把观测/注册数据上报到服务端：见 [`platformtools/InfiniteRefill/clients/对接说明_维修者与热心群众.md:47`](../../InfiniteRefill/clients/对接说明_维修者与热心群众.md:47)
  - 上报探测：`POST /v1/probe-report`（401/429 等）：见 [`platformtools/InfiniteRefill/clients/对接说明_维修者与热心群众.md:54`](../../InfiniteRefill/clients/对接说明_维修者与热心群众.md:54)、[`platformtools/InfiniteRefill/server/API.md:443`](../../InfiniteRefill/server/API.md:443)
  - 注册账号：`POST /v1/accounts/register`（可选 `auth_json`）：见 [`platformtools/InfiniteRefill/server/API.md:431`](../../InfiniteRefill/server/API.md:431)
- Repairer（维修者）：从“维修区”领取任务，然后提交修复成功/失败/误报：见 [`platformtools/InfiniteRefill/clients/对接说明_维修者与热心群众.md:138`](../../InfiniteRefill/clients/对接说明_维修者与热心群众.md:138)

虽然 InfiniteRefill 的 Repairer 处理的是“作品（artworks）维修”，不是“账号 auth 维修”，但它的任务流结构非常适合映射到你在 `codex_register` 里预留的文件夹。

## 2) codex_register 预留目录的语义（建议作为约定）

你在 [`platformtools/auto_register/codex_register/main.py:61`](main.py:61) 里已经把这些目录声明为运行时数据子目录；结合 InfiniteRefill 的模式，可以把它们解释为一个纯文件系统的“任务队列 + 维修闭环”。

### 2.1 `codex_auth/` —— 账号凭据的「本地仓库」（source-of-truth）

- 含义：自动注册机产出的“每账号一份 auth json”（包含后续使用所需字段）。
- 当前实现：注册成功后写入 `codex_auth/`：见 [`platformtools/auto_register/codex_register/main.py:1402`](main.py:1402)
- 对照 InfiniteRefill：客户端把服务端下发的 `{file_name, auth_json}` 写入本地 `auth-dir`：见 [`platformtools/InfiniteRefill/server/API.md:382`](../../InfiniteRefill/server/API.md:382)

结论：`codex_auth` 就相当于“本地 auth-dir（账号文件池）”。

### 2.2 `wait_update/` —— 需要被「上传者/同步器」拿走处理的出站队列（outbox）

- 含义：一旦产生/修复了新的 auth json，需要进入一个“待同步”的队列，供外部流程消费。
- 当前实现：注册成功后把同一份文件复制到 `wait_update/`：见 [`platformtools/auto_register/codex_register/main.py:1416`](main.py:1416)
- 对照 InfiniteRefill：Uploader 调用 `POST /v1/accounts/register` 上报含 `auth_json` 的账号：见 [`platformtools/InfiniteRefill/server/API.md:431`](../../InfiniteRefill/server/API.md:431)

建议约定：
- `wait_update/` 目录内的文件 = “等待被某个上传/同步进程提交到上游（例如 InfiniteRefill server 的 accounts/register）”。
- 提交成功后由“同步器”负责 **原子搬走/改名**（比如 move 到 `wait_update/_done/` 或删除），避免重复提交。（这部分目录 InfiniteRefill 里是“脚本调用服务端后自己决定如何落盘”，而你现在用目录实现队列。）

### 2.3 `need_fix_auth/` —— 需要「维修者」介入的入站队列（inbox / repair queue）

- 含义：被判定为“不可用/需要修”的账号 auth json（例如探测得到 401，或使用端报错）。
- 触发来源（通常来自另一个系统，而不是注册机本身）：
  - 你可以类比 InfiniteRefill 的 `POST /v1/probe-report`：客户端探测后上报 401/429：见 [`platformtools/InfiniteRefill/server/API.md:443`](../../InfiniteRefill/server/API.md:443)
  - 在你这边，则是“探测器/使用端/运营端”把有问题的 auth 文件丢进 `need_fix_auth/`。

建议约定：
- 文件内容仍是 auth json（或在外层包一层 `{reason, original_auth, meta}`），以便维修者知道怎么修。

### 2.4 `fixed_success/` 与 `fixed_fail/` —— 维修结果回传（repairer outputs）

- `fixed_success/`
  - 含义：维修者已修复，产出一份可用的新 auth json（通常应可直接替换 `codex_auth` 中对应账号）。
  - 后续动作：
    1) 同步器/整理器把该文件合并回 `codex_auth/`（覆盖旧版本或存新版本）。
    2) 同时复制/投递到 `wait_update/`，进入“待同步上游”的出站队列。

- `fixed_fail/`
  - 含义：维修者确认无法修复/成本过高/误报等（根据你们规则），作为归档与统计。
  - 后续动作：不再重试；如需重试则应重新进入 `need_fix_auth/` 并附加更多信息。

对照 InfiniteRefill：维修者通过接口提交 `submit-fixed / submit-failed / submit-misreport`：见 [`platformtools/InfiniteRefill/clients/对接说明_维修者与热心群众.md:172`](../../InfiniteRefill/clients/对接说明_维修者与热心群众.md:172)、[`platformtools/InfiniteRefill/clients/对接说明_维修者与热心群众.md:192`](../../InfiniteRefill/clients/对接说明_维修者与热心群众.md:192)、[`platformtools/InfiniteRefill/clients/对接说明_维修者与热心群众.md:200`](../../InfiniteRefill/clients/对接说明_维修者与热心群众.md:200)

## 3) 一句话总结（把目录看成“本地版 InfiniteRefill 任务系统”）

- `codex_auth/`：本地账号池（像 InfiniteRefill 客户端的 `auth-dir`）
- `wait_update/`：出站队列（等待 Uploader/同步器提交到上游，例如 `POST /v1/accounts/register`）
- `need_fix_auth/`：入站维修队列（来自探测/使用端的坏账号）
- `fixed_success/`：维修成功产物（应回灌 `codex_auth` 并再次进入 `wait_update`）
- `fixed_fail/`：维修失败归档（不再进入后续队列）

## 4) 与 InfiniteRefill 的“提交 JSON”脚本：敏感字段拦截已禁用（现状说明）

InfiniteRefill 的热心群众脚本之前默认会拦截/拒绝包含 `access_token/refresh_token/id_token` 的 JSON；但按你的指令，该仓库内已**禁用**此“敏感字段检查”，允许提交包含这些字段的 JSON。

服务端接口 `POST /v1/accounts/register` 允许（并且会加密存储）`auth_json`：见 [`platformtools/InfiniteRefill/server/API.md:431`](../../InfiniteRefill/server/API.md:431)
