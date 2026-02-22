//! 类型定义

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 统一聊天请求（最小可用字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyChatRequest {
    pub model: String,
    pub messages: Vec<ProxyMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// 统一消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMessage {
    pub role: String,
    pub content: String,
}

impl ProxyMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

/// 目标代理类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlackMagicProxyTarget {
    CliProxyApi,
    NewApi,
    CcSwitch,
    ClaudeCodeRouter,
    IFlow,
    Antigravity,
    GeminiCli,
    Vertex,
    /// Google AI Studio (generativelanguage.googleapis.com) - API Key 认证
    GoogleAIStudio,
    /// Vertex Compat - 第三方转发站代理（如 zenmux.ai）
    VertexCompat,
}

/// 反代接口形态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProxyExposureKind {
    /// 传统 HTTP API（JSON）
    Api,
    /// 鉴权中转（通过 token/cookie/session 对上游认证）
    Auth,
    /// WebSocket 双工流
    WebSocket,
    /// CLI 进程中转
    Cli,
}

/// 单个"反代形态"配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyExposure {
    pub kind: ProxyExposureKind,
    /// endpoint path 或 ws path；CLI 下可留空
    pub path: String,
    /// method（HTTP 使用），WS/CLI 可忽略
    pub method: String,
    /// 鉴权头（如 Authorization / x-api-key）
    pub auth_header: Option<String>,
    /// 鉴权前缀（如 Bearer ）
    pub auth_prefix: Option<String>,
    /// CLI 命令（仅 CLI 模式）
    pub cli_command: Option<String>,
    /// CLI 参数（仅 CLI 模式）
    pub cli_args: Vec<String>,
    /// 备注
    pub notes: String,
}

/// 单个代理规格描述（按项目聚合多个暴露方式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackMagicProxySpec {
    pub target: BlackMagicProxyTarget,
    pub default_base_url: String,
    pub exposures: Vec<ProxyExposure>,
    pub notes: String,
}

/// 准备后的 HTTP 调用描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPreparedHttpCall {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: serde_json::Value,
}

/// 准备后的 WebSocket 调用描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPreparedWsCall {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    /// 可用于 ws 握手后发送首帧初始化数据
    pub init_payload: serde_json::Value,
}

/// 准备后的 CLI 调用描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPreparedCliCall {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    /// 可用于 stdin 注入
    pub input_payload: String,
}

/// 统一的"准备结果"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyPreparedCall {
    Http(ProxyPreparedHttpCall),
    WebSocket(ProxyPreparedWsCall),
    Cli(ProxyPreparedCliCall),
}
