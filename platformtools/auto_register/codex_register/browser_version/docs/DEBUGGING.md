# 调试排障指南

## 快速调试启动

```bash
cd browser_version

# 设置调试环境变量
set HEADLESS=0                        # 显示浏览器
set CONCURRENCY=1                     # 单 Worker
set MAX_ATTEMPTS_PER_WORKER=1         # 单次尝试
set KEEP_BROWSER_OPEN_ON_FAIL=1       # 失败保留浏览器
set DUMP_PAGE_BODY=1                  # 输出页面文本
set KEEP_ERROR_ARTIFACTS=1            # 保存错误截图
set DISABLE_PROXY=1                   # 禁用代理
set REGISTER_PROXY_REQUIRED=0         # 允许直连
set ENABLE_PROBE=0                    # 禁用探针
set ENABLE_REPAIRER=0                 # 禁用修缮
set MAILBOX_VERBOSE=1                 # 邮箱详细日志

python main.py 2>&1 | tee debug.log
```

## 常见问题与解决方案

### 1. MailCreate 401 认证错误

**错误信息**：
```
[Worker 1] [x] new_address failed: 401 你已启用私有站点密码,请提供密码
```

**原因**：`MAILCREATE_CUSTOM_AUTH` 为空。

**检查步骤**：
1. 确认 `platformtools/.dev.vars` 文件存在且包含 `MAILCREATE_CUSTOM_AUTH=...`
2. 运行快速诊断：
   ```python
   import sys, os
   sys.path.insert(0, r'platformtools路径')
   from _shared.dev_vars import load_platformtools_dev_vars
   d = load_platformtools_dev_vars(start_dir=r'browser_version路径')
   print('AUTH:', d.get('MAILCREATE_CUSTOM_AUTH'))
   ```
3. 检查 `_REPO_ROOT`（main.py line 955）是否正确指向 `platformtools/` 的父级

**历史 Bug**：`_REPO_ROOT` 少退了一级，`from platformtools._shared.dev_vars import ...` 静默失败。已修复为 4 级 `..`。

### 2. 邮箱输入框找不到

**错误信息**：
```
email input not found after retries
```

**可能原因**：
- Cloudflare Turnstile 挑战阻塞页面
- OpenAI 更新了 HTML 结构，动态 ID 变化
- 页面加载超时

**排查**：
1. 检查 `data/error/` 目录中的 dump 文件，查看当时页面文本
2. 增加 `DEBUG_EMAIL_WAIT_ROUNDS` 和 `DEBUG_EMAIL_PREWAIT_SECONDS`
3. 使用 `HEADLESS=0` 直观观察浏览器状态

### 3. 密码阶段被 Cloudflare 拦截

**错误信息**：
```
blocked challenge page before password step
```

**说明**：Cloudflare 检测到自动化行为，弹出人机验证。

**应对**：
- 增加 `DEBUG_CHALLENGE_GRACE_ROUNDS`（默认 6 轮，每轮 2 秒）
- 使用 `undetected-chromedriver`（默认已启用）
- 更换代理 IP
- 降低并发数

### 4. OTP 验证码超时

**错误信息**：
```
otp submitted but page did not advance
```

**可能原因**：
- 邮件到达延迟（>120 秒）
- 验证码已过期
- MailCreate 服务异常

**排查**：
1. 检查 `MAILBOX_VERBOSE=1` 日志中的 `mailcreate poll` 消息
2. 手动调用 MailCreate API 验证连通性
3. 增加 `OTP_TIMEOUT_SECONDS`

### 5. Callback URL 超时

**错误信息**：
```
Blocked: Timeout waiting for callback URL to localhost.
```

**可能原因**：
- `/about-you` 页面生日验证失败
- Codex 同意页的"继续"按钮未匹配
- 意外跳转到 `openai.com/policies` 页面

**排查**：
1. 查看 dump 文件中 `about_you_force_submit` 和 `continue_missing_no_enter` 的页面文本
2. `KEEP_BROWSER_OPEN_ON_FAIL=1`，查看浏览器停在哪一步
3. 检查 `_click_final_continue_if_present()` 的 XPath 是否需要更新

### 6. Chrome 版本不匹配

**错误信息**：
```
[driver] uc init failed, fallback selenium webdriver.Chrome: ...
```

**解决**：
- 设置 `CHROME_VERSION_MAIN=你的Chrome主版本号`
- 检查 Chrome 版本：打开 `chrome://version/`

### 7. 代理无法连接

**错误信息**：
```
[Worker 1] [x] ... (准备换IP重试)
```

**解决**：
- 检查 `data/proxies.txt` 中的代理格式
- 临时禁用代理测试：`DISABLE_PROXY=1`
- 确认代理支持 HTTPS

## 日志分析

### 关键日志标记

| 标记 | 含义 |
|------|------|
| `[register] start` | 注册流程开始 |
| `[mailbox] mailbox_direct=true` | 邮箱创建开始 |
| `[ui] email input found` | 邮箱输入框定位成功 |
| `[ui] reach password input` | 密码输入框定位成功 |
| `[otp] otp stage ready` | 进入验证码阶段 |
| `[mail] got verification code` | OTP 获取成功 |
| `[about-you] force submit` | 姓名/生日提交 |
| `[consent] fallback click` | Codex 同意页点击 |
| `Success Callback URL Captured` | Callback 捕获成功 |
| `[Worker N] [✓]` | 注册成功 |
| `[Worker N] [x]` | 注册失败 |

### 输出重定向

PowerShell：
```powershell
python main.py 2>&1 | Out-File -Encoding utf8 debug.log
```

Bash：
```bash
python main.py 2>&1 | tee debug.log
```

## 错误产物目录

`data/error/{INSTANCE_ID}/` 中按时间戳和类型保存：

| 文件类型 | 说明 |
|----------|------|
| `*_body.txt` | 页面 `innerText` 前 4000 字符 |
| `*_screenshot.png` | 页面截图（如果可用） |
| `*_page.html` | 页面完整 HTML |

## 生产环境调优建议

1. **并发数**：单容器建议 `CONCURRENCY=1`，避免共享内存竞争
2. **多实例**：通过增加容器实例数提升吞吐量
3. **禁用调试功能**：生产环境设 `DUMP_PAGE_BODY=0`、`KEEP_ERROR_ARTIFACTS=0` 减少 I/O
4. **代理轮转**：配置多个代理，设置合理的冷却时间避免 IP 被封
5. **日志精简**：设 `MAILBOX_VERBOSE=0`、`DEBUG_TRACE=0`
