# nl_llm

A high-performance, unified asynchronous Rust LLM Client library that supports multi-provider integration, advanced concurrency control, and stream completion processing.

## 🔥 Key Features

- **Unified Protocol Abstraction**: Work flawlessly across OpenAI, Claude, Gemini, Qwen, Moonshot, Vertex, and other API definitions utilizing unified `PrimitiveRequest`.
- **Advanced Concurrency & Metrics**: Fine-grained RPS/Concurrency locks, auto-throttle, and usage metric collections built-in.
- **Extensive Auth Support**: Built-in support for Raw API Key, Google Application Credentials, and zero-configuration Web OAuth flows via Device Code caching.
- **Provider Multi-tier Matrix**: Strictly decouples providers into precise context profiles ensuring explicit UI consumption without configuration drift.

## 🔌 Provider & Authentication Matrix (Auth Isolation)

To provide the cleanest "What You See Is What You Get" integration interface, `nl_llm` inherently decouples service permutations. Instead of clunky runtime toggles, you explicitly initialize the EXACT business requirement via `LlmClient::from_preset("ID")`. 

This guarantees explicit separation between **Authentication Methods** (API Key vs Web OAuth) and **Service Tiers** (Base vs Coding models).

### 🌑 Kimi (Moonshot) Matrix
| Preset ID | Routing | Auth Type | Primary Model Defaults |
|-------------|----------|----------------|---------------------|
| `kimi` | `api.moonshot.cn` | Standard API Key | General (`moonshot-32k`, `kimi-k2.5`) |
| `kimi_coding` | `api.kimi.com/v1` | Standard API Key | Code & Dev (`kimi-for-coding`) |
| `kimi_oauth` | `auth.kimi.com` | **Web OAuth (Device)** | **Zero-Key** browser interactive authorization. |

### 🔵 Qwen (通义千问) Matrix
| Preset ID | Routing | Auth Type | Primary Model Defaults |
|-------------|----------|----------------|---------------------|
| `qwen` | `dashscope` | Standard API Key | General (`qwen-plus`, `qwen-vl`) |
| `qwen_coder` | `dashscope` | Standard API Key | Code & Dev (`qwen2.5-coder-32b-instruct`) |
| `qwen_oauth`| `portal.qwen.ai` | **Web OAuth (Device)** | **Zero-Key** browser interactive authorization. |

> *For testing, evaluate the examples mapping exactly to these presets under `./examples/[preset_id]/`.*

## 🚀 Quick Start

```rust
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 明确声明我们需要无需 API KEY 即可白嫖 Web Token 的通道
    let client = LlmClient::from_preset("qwen_oauth")
        .expect("OAuth preset must exist")
        .with_qwen_oauth("./qwen_cache.json") // 这行命令会拦截终端并弹出浏览器扫码授权
        .build();

    let req = PrimitiveRequest::single_user_message("Write a hello world in Rust");
    let resp = client.complete(&req).await?;
    
    println!("{}", resp.content);
    Ok(())
}
```
