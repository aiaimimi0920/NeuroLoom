# 环境变量参考手册

## 核心配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `CONCURRENCY` | `1` | 并发 Worker 数量 |
| `MAX_ATTEMPTS_PER_WORKER` | `0`（无限） | 每个 Worker 最大尝试次数 |
| `HEADLESS` | `1` | 是否无头模式。`0`=显示浏览器窗口 |
| `INSTANCE_ID` | 主机名 | 实例唯一标识，用于结果分片目录名 |

## 浏览器配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `USE_UNDETECTED_CHROMEDRIVER` | `1` | 是否使用 undetected-chromedriver |
| `CHROME_VERSION_MAIN` | 自动检测 | Chrome 主版本号（如 `134`） |
| `BROWSER_WINDOW_SIZE` | `500,600` | 浏览器窗口尺寸 `宽,高` |
| `ANONYMOUS_MODE` | `0` | 是否启用无痕模式 |

## 代理配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `DISABLE_PROXY` | `0` | `1`=强制不使用代理 |
| `REGISTER_PROXY_REQUIRED` | `1` | `0`=无代理时允许直连 |
| `PROXY_ROTATE_SECONDS` | — | 代理轮转间隔（秒） |

## 邮箱服务

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `MAILBOX_PROVIDER` | `auto` | 邮箱提供商：`auto`/`mailcreate`/`gptmail`/`mailtm` |
| `MAILCREATE_BASE_URL` | `.dev.vars` | MailCreate 服务地址 |
| `MAILCREATE_CUSTOM_AUTH` | `.dev.vars` | MailCreate 认证密钥 |
| `MAILCREATE_DOMAIN` | — | 指定邮箱域名 |
| `MAIL_DOMAIN_HEALTH_ORDER` | — | 域名健康优选顺序（逗号分隔） |
| `GPTMAIL_BASE_URL` | `https://mail.chatgpt.org.uk` | GPTMail 服务地址 |
| `GPTMAIL_API_KEY` | — | GPTMail API Key |
| `GPTMAIL_KEYS_FILE` | — | GPTMail 多 Key 文件路径 |
| `GPTMAIL_PREFIX` | — | GPTMail 邮箱前缀 |
| `GPTMAIL_DOMAIN` | — | GPTMail 邮箱域名 |
| `MAILTM_API_BASE` | `https://api.mail.tm` | Mail.tm API 地址 |
| `MAILBOX_VERBOSE` | `0` | `1`=启用邮箱操作详细日志 |
| `OTP_TIMEOUT_SECONDS` | `120` | OTP 验证码等待超时 |

## 调度器配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `SCHED_FAILURE_PENALTY` | `15` | 每次失败扣减的优先级分数 |
| `SCHED_DECAY_HALF_LIFE_SECONDS` | `300` | 失败计数半衰期（秒） |
| `GPT_TEST_BAN_THRESHOLD` | `3` | `gpt-test` 当日封禁阈值 |
| `SCHED_PRIO_GPTMAIL_TEST` | `100` | `gptmail_test` 基础优先级 |
| `SCHED_PRIO_MAILCREATE` | `80` | `mailcreate` 基础优先级 |
| `SCHED_PRIO_GPTMAIL_PAID` | `60` | `gptmail_paid` 基础优先级 |
| `SCHED_PRIO_MAILTM` | `40` | `mailtm` 基础优先级 |
| `GPTMAIL_PAID_KEY_EXHAUST_THRESHOLD` | `10` | 付费 key 连续非网络失败次数达此阈值永久废弃 |

## 流量节省

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `BLOCK_IMAGES` | `2` | `2`=拦截图片，`1`=允许 |
| `BLOCK_CSS` | `2` | `2`=拦截 CSS，`1`=允许 |
| `BLOCK_FONTS` | `2` | `2`=拦截字体，`1`=允许 |
| `BLOCK_GOOGLE_OPT_GUIDE` | `2` | `2`=屏蔽 Google 优化指南 |
| `BLOCK_NOISY_HOSTS` | `2` | `2`=屏蔽 Chrome 后台噪音请求 |

## 调试参数

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `KEEP_BROWSER_OPEN_ON_FAIL` | `0` | `1`=失败时保留浏览器窗口 |
| `DUMP_PAGE_BODY` | `0` | `1`=关键步骤 dump 页面 innerText |
| `KEEP_ERROR_ARTIFACTS` | `0` | `1`=保存错误截图和 HTML |
| `DEBUG_TRACE` | `0` | `1`=启用详细调试跟踪 |
| `DEBUG_EMAIL_PREWAIT_SECONDS` | `4.0` | 非无头模式下邮箱填写前等待 |
| `DEBUG_EMAIL_WAIT_ROUNDS` | `3`/`1` | 邮箱输入框查找轮次（调试/正常） |
| `DEBUG_EMAIL_RETRY_SLEEP_SECONDS` | `2.0` | 邮箱输入框重试间隔 |
| `DEBUG_PASSWORD_WAIT_ROUNDS` | `3`/`1` | 密码输入框查找轮次 |
| `DEBUG_PASSWORD_RETRY_SLEEP_SECONDS` | `2.0` | 密码输入框重试间隔 |
| `DEBUG_CHALLENGE_GRACE_ROUNDS` | `6`/`3` | Cloudflare 人机验证等待轮次 |
| `SMART_WAIT_CHALLENGE_GRACE_SECONDS` | `12` | smart_wait 检测到 challenge 时的宽限时间 |

## 运行控制

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `SLEEP_MIN` | — | 两次尝试之间最小休眠（秒） |
| `SLEEP_MAX` | — | 两次尝试之间最大休眠（秒） |
| `ENABLE_PROBE` | `1` | 是否启用健康探针守护线程 |
| `ENABLE_REPAIRER` | `1` | 是否启用 Token 修缮守护线程 |
| `RESULTS_SHARD_SIZE` | `200` | 结果分片大小（每片多少条） |

## 数据加载优先级

每个配置变量的读取顺序：
1. **系统环境变量** `os.environ.get("...")` — 最高优先级
2. **`.dev.vars` 文件** `_PLATFORMTOOLS_DEV_VARS.get("...")` — 开发变量
3. **本地 JSON 配置** `_MAILCREATE_CFG.get("...")` — 兜底

> ⚠️ **注意**: `.dev.vars` 的加载依赖 `_REPO_ROOT` 路径正确指向 `platformtools/` 的父目录，以确保 `from platformtools._shared.dev_vars import ...` 能成功导入。
