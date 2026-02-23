//! Provider Trait 定义

use async_trait::async_trait;
use futures::Stream;

use crate::primitive::PrimitiveRequest;
use crate::auth::Auth;

/// LLM Provider 统一 Trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 唯一标识
    fn id(&self) -> &str;

    /// 认证类型
    fn auth(&self) -> &Auth;

    /// 支持的模型列表
    fn supported_models(&self) -> &[&str];

    /// 将原语编译为请求体
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value;

    /// 执行请求
    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse>;

    /// 流式执行
    async fn stream(
        &self,
        body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>>;

    /// 是否需要刷新认证
    fn needs_refresh(&self) -> bool {
        false
    }

    /// 刷新认证
    async fn refresh_auth(&mut self) -> crate::Result<()> {
        Ok(())
    }
}

// ================================================================================================
// 正交分解体系 (Orthogonal Decomposition) - New Architecture
// ================================================================================================

/// 协议层 (Protocol) - 负责压包和解包
///
/// 决定请求体的 JSON 结构和如何解析目标服务器返回的结果。
pub trait Protocol: Send + Sync {
    /// 把统一的抽象原语压缩为特定平台协议所需的 JSON 体
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value;
    
    /// 从特定平台的普通 HTTP 返回体中解包还原 LlmResponse
    fn parse_response(&self, raw_text: &str) -> crate::Result<LlmResponse>;
    
    /// 从特定平台的 SSE 流事件中解包还原 LlmChunk 流
    fn parse_stream(
        &self,
        resp: reqwest::Response,
    ) -> crate::Result<BoxStream<'static, crate::Result<LlmChunk>>>;
}

/// 端点层 (Endpoint) - 负责路由拦截与身份注入
///
/// 决定把数据发向哪个 URL (Official/Forwarding)，并挂载如何验证票据 (Cookie/OAuth/APIKey)。
#[async_trait]
pub trait Endpoint: Send + Sync {
    /// 利用 Auth 提取有效门票或强制刷新的前置勾子
    async fn pre_flight(&self) -> crate::Result<()>;
    
    /// 获取路由最终派发目标（可能是经过复杂签名的动态 URL）
    fn url(&self, model: &str, is_stream: bool) -> crate::Result<String>;
    
    /// 为原生的 HTTP Request Builder 注入 `Authorization` 等凭证，和附加 Headers (如 IDE Metadata)
    fn inject_auth(&self, req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder>;

    /// （可选）在发送前最后一次魔改 Body 的机会 (如注入特殊标志 projectId, userAgent 等)
    fn decorate_body(&self, body: serde_json::Value) -> serde_json::Value {
        body
    }

    /// （可选）指示该端点是否需要刷新底层 Auth 门票
    fn needs_refresh(&self) -> bool {
        false
    }

    /// （可选）刷新底层 Auth 门票
    async fn refresh_auth(&self) -> crate::Result<()> {
        Ok(())
    }
}

/// 通用客户端 (Generic Client)
///
/// 这是 Orthogonal Decomposition 下剥离出的纯粹拼装型执行器，负责统管：
/// 发包拦截、网络错误恢复、5xx等状态码统一判定。
pub struct GenericClient<E, P>
where
    E: Endpoint,
    P: Protocol,
{
    pub id: String,
    pub endpoint: E,
    pub protocol: P,
    pub auth: Auth,
    pub supported_models: Vec<String>,
    pub http: reqwest::Client,
    /// 缓存的支持模型列表（避免重复转换，线程安全）
    pub(crate) supported_models_cache: std::sync::OnceLock<&'static [&'static str]>,
}

#[async_trait]
impl<E, P> LlmProvider for GenericClient<E, P>
where
    E: Endpoint,
    P: Protocol,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn auth(&self) -> &Auth {
        &self.auth
    }

    fn supported_models(&self) -> &[&str] {
        // 使用 OnceLock 缓存转换结果，避免每次调用都创建新 Vec
        // 由于 trait 返回 &[&str]，我们需要泄漏内存来获得 'static 生命周期
        // 但只在第一次调用时泄漏一次，后续调用复用
        self.supported_models_cache
            .get_or_init(|| {
                // 将 Vec<String> 转换为 Box<[&'static str]>，需要泄漏内存
                // 这是安全的，因为 supported_models 在 GenericClient 生命周期内一直存在
                // 并且我们只泄漏一次（通过 OnceLock 保证）
                let static_strs: Box<[&'static str]> = self.supported_models
                    .iter()
                    .map(|s| {
                        // 泄漏每个 String 来获得 &'static str
                        unsafe { std::mem::transmute::<&str, &'static str>(s.as_str()) }
                    })
                    .collect();
                // 再泄漏 Box 本身以获得 &'static [&'static str]
                Box::leak(static_strs)
            })
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        // 协议层先压包
        let mut body = self.protocol.compile(primitive);
        // 端点层负责最终修饰注入
        body = self.endpoint.decorate_body(body);
        // 保存 model 配置供发包路由使用
        if let Some(obj) = body.as_object_mut() {
            if obj.get("_gw_model").is_none() {
                obj.insert("_gw_model".to_string(), serde_json::json!(&primitive.model));
            }
        }
        body
    }

    async fn complete(&self, mut body: serde_json::Value) -> crate::Result<LlmResponse> {
        self.endpoint.pre_flight().await?;
        
        // 提取被存下的路由级别 model 字段
        let model = body.as_object_mut().and_then(|obj| obj.remove("_gw_model"))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let url = self.endpoint.url(&model, false)?;
        let mut req = self.http.post(&url).header("Content-Type", "application/json");
        req = self.endpoint.inject_auth(req)?;
        
        let resp = req.json(&body).send().await.map_err(|e| crate::Error::Http(e.to_string()))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            let status_code = status.as_u16();
            return Err(crate::Error::Provider(ProviderError::from_http_status(
                status_code,
                format!("{} generateContent failed ({}): {}", self.id, status_code, text.trim()),
            )));
        }

        self.protocol.parse_response(&text)
    }

    async fn stream(
        &self,
        mut body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        self.endpoint.pre_flight().await?;
        
        let model = body.as_object_mut().and_then(|obj| obj.remove("_gw_model"))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let url = self.endpoint.url(&model, true)?;
        let mut req = self.http.post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream");
            
        req = self.endpoint.inject_auth(req)?;
        
        let resp = req.json(&body).send().await.map_err(|e| crate::Error::Http(e.to_string()))?;
        let status = resp.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(ProviderError::from_http_status(
                status_code,
                format!("{} stream failed ({}): {}", self.id, status_code, text.trim()),
            )));
        }

        // 把 SSE 解包能力下放给负责方块的协议实现
        self.protocol.parse_stream(resp)
    }

    fn needs_refresh(&self) -> bool {
        self.endpoint.needs_refresh()
    }

    async fn refresh_auth(&mut self) -> crate::Result<()> {
        self.endpoint.refresh_auth().await
    }
}


/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// 响应内容
    pub content: String,
    /// 工具调用
    pub tool_calls: Vec<ToolCall>,
    /// 使用统计
    pub usage: Usage,
    /// 停止原因
    pub stop_reason: StopReason,
}

/// LLM 流式块
#[derive(Debug, Clone)]
pub struct LlmChunk {
    /// 增量内容
    pub delta: ChunkDelta,
    /// 使用统计（最后一块可能有）
    pub usage: Option<Usage>,
}

/// 增量内容类型
#[derive(Debug, Clone)]
pub enum ChunkDelta {
    /// 文本
    Text(String),
    /// 工具调用
    ToolCall {
        id: String,
        name: String,
        delta: String,
    },
    /// 思考内容
    Thinking(String),
}

/// 停止原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// 正常结束
    EndTurn,
    /// 工具调用
    ToolUse,
    /// 达到最大 token
    MaxTokens,
    /// 遇到停止序列
    StopSequence,
}

/// 工具调用
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// 调用 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 参数
    pub arguments: serde_json::Value,
}

/// 使用统计
#[derive(Debug, Clone, Default)]
pub struct Usage {
    /// 输入 token 数
    pub input_tokens: u64,
    /// 输出 token 数
    pub output_tokens: u64,
    /// 思考 token 数（如果有）
    pub thinking_tokens: Option<u64>,
}

/// Provider 执行错误，带有重试信号
#[derive(Debug, Clone)]
pub struct ProviderError {
    /// 错误消息
    pub message: String,
    /// 是否应该在同一 Provider 重试
    pub retryable: bool,
    /// 是否应该触发跨 Provider 降级
    pub should_fallback: bool,
    /// 建议的重试延迟（毫秒）
    pub retry_after_ms: Option<u64>,
}

impl ProviderError {
    /// 构造一个不可重试、不支持降级的基本错误
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            should_fallback: false,
            retry_after_ms: None,
        }
    }

    /// 构造一个可重试的错误
    pub fn retryable(
        message: impl Into<String>,
        should_fallback: bool,
        retry_after_ms: Option<u64>,
    ) -> Self {
        Self {
            message: message.into(),
            retryable: true,
            should_fallback,
            retry_after_ms,
        }
    }

    /// 从 HTTP 状态码自动推导是否可重试
    pub fn from_http_status(status: u16, message: impl Into<String>) -> Self {
        let msg = message.into();
        // 429 Too Many Requests 或者 5xx 服务器内部错误 -> 可重试+应降级
        if status == 429 || status >= 500 {
            Self::retryable(msg, true, None)
        } else {
            Self::fail(msg)
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProviderError {}


/// 辅助宏：创建 GenericClient 并自动初始化缓存字段
#[macro_export]
macro_rules! generic_client {
    (
        id: $id:expr,
        endpoint: $endpoint:expr,
        protocol: $protocol:expr,
        auth: $auth:expr,
        supported_models: $models:expr,
        http: $http:expr
    ) => {
        $crate::provider::GenericClient {
            id: $id,
            endpoint: $endpoint,
            protocol: $protocol,
            auth: $auth,
            supported_models: $models,
            http: $http,
            supported_models_cache: std::sync::OnceLock::new(),
        }
    };
}

/// BoxStream 类型别名
pub type BoxStream<'a, T> = std::pin::Pin<Box<dyn Stream<Item = T> + Send + 'a>>;
