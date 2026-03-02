# Codex Register 代理强制化实施说明（邮箱直连例外）

## 1. 目标与范围

本次改造目标：

- 注册主流程（browser + protocol）**必须走代理**。
- `HEADLESS=0` 不再允许注册直连。
- 邮箱链路（MailCreate/GPTMail）继续直连，并提供可观测日志。
- 不改变 repairer/probe/uploader 既有行为。

范围文件：

- [`platformtools/auto_register/codex_register/main.py`](platformtools/auto_register/codex_register/main.py)
- [`platformtools/auto_register/codex_register/browser_version/main.py`](platformtools/auto_register/codex_register/browser_version/main.py)
- [`platformtools/auto_register/codex_register/protocol_main.py`](platformtools/auto_register/codex_register/protocol_main.py)

---

## 2. 已实施改造点

### 2.1 注册主流程新增强制代理开关

三条主链路均新增并启用：

- [`REGISTER_PROXY_REQUIRED`](platformtools/auto_register/codex_register/main.py:619)
- [`REGISTER_PROXY_REQUIRED`](platformtools/auto_register/codex_register/browser_version/main.py:204)
- [`REGISTER_PROXY_REQUIRED`](platformtools/auto_register/codex_register/protocol_main.py:286)

默认值为 `1`（开启）。

### 2.2 移除 HEADLESS=0 注册直连兜底

- 在 [`worker()`](platformtools/auto_register/codex_register/main.py:3634) 中取消 `HEADLESS=0` 直连分支，改为无代理即失败并重试。
- 在 [`worker()`](platformtools/auto_register/codex_register/browser_version/main.py:4055) 中将 `_pick_proxy(... force_direct=False)` 固定化，不再根据 `HEADLESS` 降级直连。

### 2.3 protocol 链路代理一致性与观测

- 在 [`register_protocol()`](platformtools/auto_register/codex_register/protocol_main.py:1038) 中增加无代理即失败守卫。
- 在 `trace` 日志增加 `proxy_id`：[`_log()` 输出点](platformtools/auto_register/codex_register/protocol_main.py:1065)。
- worker 启动日志增加 `proxy_id`：[`use_proxy/proxy_id` 输出](platformtools/auto_register/codex_register/protocol_main.py:1278)。

### 2.4 邮箱链路直连显式隔离与标记

在主链路中增加日志标记：

- [`get_email()`](platformtools/auto_register/codex_register/main.py:1251) 增加 `mailbox_direct=true`。
- [`get_oai_code()`](platformtools/auto_register/codex_register/main.py:1267) 增加 `mailbox_direct=true`。
- [`get_email()`](platformtools/auto_register/codex_register/browser_version/main.py:1610) 增加 `mailbox_direct=true`。
- [`register_protocol()`](platformtools/auto_register/codex_register/protocol_main.py:1069) 与验证码拉取点 [`wait_openai_code()` 调用前](platformtools/auto_register/codex_register/protocol_main.py:1148) 增加 `mailbox_direct=true`。

---

## 3. 回滚点

若出现不可接受回归，可按最小影响回滚：

1. 将环境变量 `REGISTER_PROXY_REQUIRED=0`（快速软回滚）。
2. 若需彻底恢复旧行为，回退以下函数改动：
   - [`worker()`](platformtools/auto_register/codex_register/main.py:3634)
   - [`worker()`](platformtools/auto_register/codex_register/browser_version/main.py:4044)
   - [`register_protocol()`](platformtools/auto_register/codex_register/protocol_main.py:1038)
   - [`worker()`](platformtools/auto_register/codex_register/protocol_main.py:1234)

---

## 4. 上线核对清单

### 4.1 配置核对

- `data/proxies.txt` 非空，且格式为 `http://user:pass@host:port`。
- 注册容器环境中确认 `REGISTER_PROXY_REQUIRED=1`。
- `REGISTER_FLOW_MODE` 与预期一致（browser/protocol）。

### 4.2 日志核对

启动后确认出现：

- 注册 worker 日志出现 `use_proxy=...`，且无 `HEADLESS=0 强制直连` 文案。
- 若无可用代理，出现 `register_proxy_required no_proxy_available`。
- protocol 模式日志出现 `proxy_id=` 与 `loc/ip`。
- 邮箱步骤日志出现 `mailbox_direct=true`。

### 4.3 功能核对

- 有代理时可正常完成注册并产出 auth 文件。
- 无代理时注册不直连，按预期快速失败重试。
- 邮箱链路仍可取地址和验证码。

---

## 5. 验收建议（可执行）

1. 正常路径：
   - 保持有效代理池，运行一轮注册，观察成功与日志字段齐全。
2. 失败路径：
   - 临时清空 `proxies.txt`，确认仅出现 `register_proxy_required` 失败，不发起注册直连。
3. 邮箱直连回归：
   - 观察 `mailbox_direct=true` 相关日志仍出现，验证码拉取正常。

---

## 6. 语法检查结果

已执行：

- [`python -m py_compile`](platformtools/auto_register/codex_register/main.py)

目标文件编译通过：

- [`platformtools/auto_register/codex_register/main.py`](platformtools/auto_register/codex_register/main.py)
- [`platformtools/auto_register/codex_register/browser_version/main.py`](platformtools/auto_register/codex_register/browser_version/main.py)
- [`platformtools/auto_register/codex_register/protocol_main.py`](platformtools/auto_register/codex_register/protocol_main.py)
