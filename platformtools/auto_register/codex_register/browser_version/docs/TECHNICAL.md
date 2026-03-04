# 技术方案与反检测策略

## 1. 浏览器指纹与反检测

### 1.1 undetected-chromedriver

默认启用 `undetected-chromedriver`（`USE_UNDETECTED_CHROMEDRIVER=1`），它通过以下方式绕过 Selenium 检测：
- 修改 ChromeDriver 二进制文件中的 `$cdc_` 变量名
- 避免使用 Chrome DevTools Protocol 的特征标志
- 对抗 `navigator.webdriver` 指纹
- 自动匹配 Chrome 版本（可通过 `CHROME_VERSION_MAIN` 手动指定）

### 1.2 WebDriver 属性隐藏

即使 undetected-chromedriver 已处理，脚本额外注入 CDP 脚本：

```javascript
Object.defineProperty(navigator, 'webdriver', {
    get: () => undefined
})
```

### 1.3 自动化特征禁用

```
--disable-blink-features=AutomationControlled
```

### 1.4 User-Agent 伪装

固定 User-Agent 为标准 Chrome/Linux 头：
```
Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36
```

## 2. 流量节省（Traffic Saver）

在有限代理带宽下减少不必要的流量消耗。

### 2.1 资源拦截（默认全部开启）

| 类型 | 环境变量 | 默认值 | 拦截扩展名 |
|------|----------|--------|-----------|
| 图片 | `BLOCK_IMAGES` | 2 (开) | png, jpg, jpeg, gif, webp, avif, svg, ico |
| CSS  | `BLOCK_CSS` | 2 (开) | css |
| 字体 | `BLOCK_FONTS` | 2 (开) | woff, woff2, ttf, otf, eot |

**实现方式**：
1. Chrome prefs（`profile.managed_default_content_settings`）
2. CDP `Network.setBlockedURLs`（更可靠）
3. Blink settings（`imagesEnabled=false`）

### 2.2 后台流量屏蔽

通过 `--host-resolver-rules` 将 Chrome 后台请求重定向到 `127.0.0.1`：

| 环境变量 | 屏蔽目标 |
|----------|---------|
| `BLOCK_GOOGLE_OPT_GUIDE=2` | `optimizationguide-pa.googleapis.com` |
| `BLOCK_NOISY_HOSTS=2` | `update.googleapis.com`, `browser-intake-datadoghq.com`, `*.gvt1.com`, `*.cloudflarestream.com` |

### 2.3 Chrome 后台功能禁用

```
--disable-background-networking       # 禁用后台网络
--disable-sync                        # 禁用同步
--disable-component-update            # 禁用组件更新
--disable-domain-reliability          # 禁用域名可靠性上报
--disable-client-side-phishing-detection
--disable-default-apps
--no-default-browser-check
--disable-features=TranslateUI        # 禁用翻译 UI
```

## 3. 代理系统

### 3.1 代理格式

```
# 无认证
IP:PORT

# 有认证（自动创建 Chrome Extension）
user:pass@IP:PORT
```

### 3.2 代理扩展（Proxy Auth Extension）

对于需要认证的代理，动态生成 Chrome 扩展：
- 创建临时目录，包含 `manifest.json` + `background.js`
- 通过 `chrome.webRequest.onAuthRequired` 注入认证信息
- 使用 `--load-extension=...` 加载

### 3.3 代理绕过

```
--proxy-bypass-list=<-loopback>;localhost;127.0.0.1
```

确保 OAuth callback (`localhost:1455`) 不走代理。

### 3.4 代理冷却与评分

代理系统通过给代理打分，按照 `_pick_proxy()` 智能选择：
- 成功注册加分
- 失败 / IP 被封扣分
- 冷却时间算法避免短时间内密集使用同一 IP

## 4. 人类行为模拟

### 4.1 延迟模拟 `_human_delay(min, max)`

在每个操作之间插入随机延迟，模拟人类操作节奏。

### 4.2 打字模拟 `_human_type(element, text)`

逐字符输入，每个字符之间添加随机延迟。

### 4.3 鼠标抖动 `_human_mouse_jitter(driver, attempts)`

使用 `ActionChains` 模拟微小的鼠标移动。

### 4.4 Page Load Strategy

```python
options.page_load_strategy = 'eager'  # 不等待所有资源下载
```

## 5. OAuth PKCE 流程

### 5.1 Code Challenge 生成

```python
code_verifier = base64url(random_bytes(32))
code_challenge = base64url(sha256(code_verifier))
```

### 5.2 Token 交换

注册完成后，浏览器被重定向到 `localhost:1455/auth/callback?code=xxx`，脚本：
1. 截获 callback URL
2. 提取 `code` 参数
3. POST `/oauth/token`，携带 `code_verifier` 完成 PKCE
4. 获取 `access_token` + `refresh_token`

### 5.3 本地回调服务器

脚本不启动实际 HTTP 服务器，而是直接监控浏览器 URL 变化。当 URL 包含 `localhost:1455` 时，从 URL 中提取 code 参数。

## 6. 邮箱服务（MailCreate）

### 6.1 架构

```
browser_version/main.py
  → mailbox_provider.py (抽象层)
    → mailcreate_client.py (HTTP 客户端)
      → MailCreate Worker (Cloudflare Worker)
        → 临时邮箱服务
```

### 6.2 域名健康优选 `_pick_mailcreate_with_health()`

- 收集 `MAILCREATE_DOMAIN` + `MAIL_DOMAIN_HEALTH_ORDER` 中的所有域名
- 随机抽样 `_MAILBOX_PICK_TRIES` 个候选
- 依次尝试，失败则继续下一个
- 所有候选失败时，回退服务端随机分配

### 6.3 验证码轮询 `wait_openai_code()`

- 默认间隔 3 秒轮询
- 超时 120 秒
- 正则匹配 6 位数字
- 支持 GPTMail 和 MailCreate 两种后端

## 7. 并发与守护进程

### 7.1 Worker 模型

```python
main()
  → ThreadPoolExecutor(max_workers=CONCURRENCY)
    → worker(worker_id=1)
    → worker(worker_id=2)
    → ...
```

每个 worker：
- 独立的浏览器实例
- 独立的代理分配
- 按 `MAX_ATTEMPTS_PER_WORKER` 控制最大尝试次数
- 无限循环直到达到最大尝试数

### 7.2 守护线程

- **Probe**（探针）：定期检测服务健康状态，`ENABLE_PROBE`
- **Repairer**（修缮者）：消费 `need_fix_auth/` 队列，为过期 Token 重新登录

## 8. 元素定位策略总结

每个表单元素都采用**多层 fallback** 定位：

```
JS sweep（遍历多个 CSS selector，检查可见性）
  ↓ 失败
固定 ID（如 _r_f_-email）
  ↓ 失败
CSS selector 通配符
  ↓ 失败
文本匹配（中文/英文兼容）
  ↓ 失败
Active element 检测
  ↓ 失败
异常抛出
```

**设计理念**：OpenAI 频繁更改 HTML 结构和动态 ID，因此元素定位必须具备充分的弹性和多层 fallback。
