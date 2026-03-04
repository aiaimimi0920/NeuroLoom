# 浏览器版 Codex 自动注册机

## 概述

通过 Selenium + undetected-chromedriver 自动化浏览器完成 OpenAI Codex 账号注册。相较于纯协议版（`protocol_main.py`），浏览器版能更好地应对 Cloudflare Turnstile、CAPTCHA 等人机验证挑战。

## 目录结构

```
browser_version/
├── main.py                # 核心代码（~5000行单文件）
├── requirements.txt       # Python 依赖
├── startup.bat            # 生产环境启动脚本
├── startup_debug.bat      # 本地调试启动脚本
├── data/
│   ├── proxies.txt        # 代理池（每行一个 IP:PORT 或 user:pass@IP:PORT）
│   ├── codex_auth/        # 注册成功后保存的 JSON Token
│   ├── wait_update/       # codex_auth 的副本（供下游消费）
│   ├── results/           # JSONL 结果分片
│   ├── error/             # 失败时的页面截图/dump
│   ├── need_fix_auth/     # 需要修缮的过期 Token
│   └── fixed_success/     # 修缮成功的 Token
└── docs/
    ├── README.md           # 本文件
    ├── FLOW.md             # 注册流程详解
    ├── TECHNICAL.md        # 技术方案与反检测策略
    ├── ENV_VARS.md         # 环境变量参考手册
    └── DEBUGGING.md        # 调试排障指南
```

## 快速开始

### 1. 安装依赖

```bash
pip install -r requirements.txt
```

核心依赖：
- `selenium` — 浏览器自动化
- `undetected-chromedriver` — 反检测 Chrome 驱动

### 2. 配置密钥

确保 `platformtools/.dev.vars` 中包含：

```ini
MAILCREATE_CUSTOM_AUTH=mc_xxxx...    # MailCreate 服务认证密钥
MAILCREATE_BASE_URL=https://mail.aiaimimi.com
```

### 3. 本地调试运行

```bash
cd browser_version
# 方式一：使用调试脚本（推荐）
startup_debug.bat

# 方式二：手动设置环境变量
set HEADLESS=0
set CONCURRENCY=1
set MAX_ATTEMPTS_PER_WORKER=1
set KEEP_BROWSER_OPEN_ON_FAIL=1
set DISABLE_PROXY=1
python main.py
```

### 4. 生产环境运行

```bash
startup.bat
```

## 输出产物

注册成功后，Token JSON 文件保存在两个位置：
- `data/codex_auth/` — 主存储
- `data/wait_update/` — 副本，供 InfiniteRefill 服务消费

## 相关文档

- [注册流程详解](FLOW.md)
- [技术方案与反检测策略](TECHNICAL.md)
- [环境变量参考手册](ENV_VARS.md)
- [调试排障指南](DEBUGGING.md)
