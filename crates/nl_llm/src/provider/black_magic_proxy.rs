//! 黑魔法代理接口聚合层
//!
//! 目标：梳理并统一四类开源项目常见的“反代输出形态”：
//! - API（HTTP JSON）
//! - Auth（鉴权代理头/票据）
//! - WebSocket（实时双工）
//! - CLI（本地命令行代理）
//!
//! 对应上游项目：CLIProxyAPI / newapi / ccswitch / Claude Code Router。

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

/// 目标代理类型（按上游项目分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlackMagicProxyTarget {
    CliProxyApi,
    NewApi,
    CcSwitch,
    ClaudeCodeRouter,
    IFlow,
    Antigravity,
    GeminiCli,
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

/// 单个“反代形态”配置
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

/// 统一的“准备结果”
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyPreparedCall {
    Http(ProxyPreparedHttpCall),
    WebSocket(ProxyPreparedWsCall),
    Cli(ProxyPreparedCliCall),
}

/// 黑魔法代理目录（用于文档、配置 UI、诊断）
pub struct BlackMagicProxyCatalog;

impl BlackMagicProxyCatalog {
    pub fn all_specs() -> Vec<BlackMagicProxySpec> {
        vec![
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::CliProxyApi,
                default_base_url: "http://127.0.0.1:3000".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "OpenAI 兼容 API 输出".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::Cli,
                        path: "".to_string(),
                        method: "".to_string(),
                        auth_header: None,
                        auth_prefix: None,
                        cli_command: Some("claude".to_string()),
                        cli_args: vec!["--print".to_string()],
                        notes: "可直接桥接本地 CLI".to_string(),
                    },
                ],
                notes: "CLI 包装 + API 暴露并存".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::NewApi,
                default_base_url: "http://127.0.0.1:3000".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "标准 API 中转".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::Auth,
                        path: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "强调多密钥/多渠道鉴权切换".to_string(),
                    },
                ],
                notes: "多渠道统一中转，鉴权策略较强".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::CcSwitch,
                default_base_url: "http://127.0.0.1:3456".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "兼容 API 入口".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::WebSocket,
                        path: "/ws/chat".to_string(),
                        method: "GET".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "适合实时会话切换/流式转发".to_string(),
                    },
                ],
                notes: "强调请求切换、容错与实时路由".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::ClaudeCodeRouter,
                default_base_url: "http://127.0.0.1:8787".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("x-api-key".to_string()),
                        auth_prefix: Some("".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Claude Code 生态常见 API 路由方式".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::WebSocket,
                        path: "/ws/claude".to_string(),
                        method: "GET".to_string(),
                        auth_header: Some("x-api-key".to_string()),
                        auth_prefix: Some("".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "用于实时任务分发与状态上报".to_string(),
                    },
                ],
                notes: "专注 Claude Code 路由与分流".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::IFlow,
                default_base_url: "https://apis.iflow.cn".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Auth,
                        path: "https://platform.iflow.cn/api/openapi/apikey".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Cookie".to_string()),
                        auth_prefix: None,
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Cookie 换取 API Key".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "标准 Chat API".to_string(),
                    },
                ],
                notes: "iFlow Cookie 自动保活".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::Antigravity,
                default_base_url: "https://cloudcode-pa.googleapis.com".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Auth,
                        path: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                        method: "GET".to_string(),
                        auth_header: None,
                        auth_prefix: None,
                        cli_command: None,
                        cli_args: vec![],
                        notes: "OAuth2 登录入口".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1internal/request".to_string(), // Approximate path, handled by provider
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Gemini Code Assist 内部接口".to_string(),
                    },
                ],
                notes: "Gemini Code Assist (Antigravity) 专用".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::GeminiCli,
                default_base_url: "https://cloudcode-pa.googleapis.com".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Auth,
                        path: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                        method: "GET".to_string(),
                        auth_header: None,
                        auth_prefix: None,
                        cli_command: None,
                        cli_args: vec![],
                        notes: "OAuth2 登录入口 (Gemini CLI)".to_string(),
                    },
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1internal:streamGenerateContent".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Gemini CLI 流式生成接口".to_string(),
                    },
                ],
                notes: "Gemini CLI (官方 CLI 凭据)".to_string(),
            },
        ]
    }

    pub fn by_target(target: BlackMagicProxyTarget) -> Option<BlackMagicProxySpec> {
        Self::all_specs().into_iter().find(|it| it.target == target)
    }
}

/// 统一代理客户端（当前阶段做“接口归一化 + 调用准备”）
pub struct BlackMagicProxyClient {
    target: BlackMagicProxyTarget,
    base_url: String,
    credential: String,
}

impl BlackMagicProxyClient {
    pub fn new(
        target: BlackMagicProxyTarget,
        base_url: impl Into<String>,
        credential: impl Into<String>,
    ) -> Self {
        Self {
            target,
            base_url: base_url.into(),
            credential: credential.into(),
        }
    }

    pub fn from_target_default(
        target: BlackMagicProxyTarget,
        credential: impl Into<String>,
    ) -> crate::Result<Self> {
        let spec = BlackMagicProxyCatalog::by_target(target).ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider("proxy target spec not found".to_string())
        })?;

        Ok(Self::new(target, spec.default_base_url, credential))
    }

    pub fn list_supported_exposures(&self) -> crate::Result<Vec<ProxyExposure>> {
        let spec = BlackMagicProxyCatalog::by_target(self.target).ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider("proxy target spec not found".to_string())
        })?;
        Ok(spec.exposures)
    }

    /// 按指定形态准备调用参数
    pub fn prepare_call(
        &self,
        exposure_kind: ProxyExposureKind,
        request: &ProxyChatRequest,
    ) -> crate::Result<ProxyPreparedCall> {
        let spec = BlackMagicProxyCatalog::by_target(self.target).ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider("proxy target spec not found".to_string())
        })?;

        let exposure = spec
            .exposures
            .iter()
            .find(|it| it.kind == exposure_kind)
            .ok_or_else(|| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "exposure kind {:?} not supported for target {:?}",
                    exposure_kind, self.target
                ))
            })?;

        match exposure.kind {
            ProxyExposureKind::Api | ProxyExposureKind::Auth => Ok(ProxyPreparedCall::Http(
                self.prepare_http_call(exposure, request)?,
            )),
            ProxyExposureKind::WebSocket => Ok(ProxyPreparedCall::WebSocket(
                self.prepare_ws_call(exposure, request)?,
            )),
            ProxyExposureKind::Cli => Ok(ProxyPreparedCall::Cli(
                self.prepare_cli_call(exposure, request)?,
            )),
        }
    }

    fn prepare_http_call(
        &self,
        exposure: &ProxyExposure,
        request: &ProxyChatRequest,
    ) -> crate::Result<ProxyPreparedHttpCall> {
        let mut headers = BTreeMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        if let Some(header) = &exposure.auth_header {
            let prefix = exposure.auth_prefix.clone().unwrap_or_default();
            headers.insert(
                header.to_ascii_lowercase(),
                format!("{}{}", prefix, self.credential),
            );
        }

        let body = serde_json::to_value(request).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("serialize request failed: {e}"))
        })?;

        Ok(ProxyPreparedHttpCall {
            method: exposure.method.clone(),
            url: normalize_url(&self.base_url, &exposure.path),
            headers,
            body,
        })
    }

    fn prepare_ws_call(
        &self,
        exposure: &ProxyExposure,
        request: &ProxyChatRequest,
    ) -> crate::Result<ProxyPreparedWsCall> {
        let mut headers = BTreeMap::new();
        if let Some(header) = &exposure.auth_header {
            let prefix = exposure.auth_prefix.clone().unwrap_or_default();
            headers.insert(
                header.to_ascii_lowercase(),
                format!("{}{}", prefix, self.credential),
            );
        }

        let init_payload = serde_json::to_value(request).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("serialize request failed: {e}"))
        })?;

        Ok(ProxyPreparedWsCall {
            url: normalize_ws_url(&self.base_url, &exposure.path),
            headers,
            init_payload,
        })
    }

    fn prepare_cli_call(
        &self,
        exposure: &ProxyExposure,
        request: &ProxyChatRequest,
    ) -> crate::Result<ProxyPreparedCliCall> {
        let command = exposure
            .cli_command
            .clone()
            .ok_or_else(|| crate::NeuroLoomError::LlmProvider("cli command missing".to_string()))?;

        let input_payload = serde_json::to_string(request).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("serialize request failed: {e}"))
        })?;

        let mut env = BTreeMap::new();
        env.insert("NEUROLOOM_PROXY_TOKEN".to_string(), self.credential.clone());
        env.insert(
            "NEUROLOOM_PROXY_TARGET".to_string(),
            format!("{:?}", self.target),
        );

        Ok(ProxyPreparedCliCall {
            command,
            args: exposure.cli_args.clone(),
            env,
            input_payload,
        })
    }
}

fn normalize_url(base_url: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/'),
    )
}

fn normalize_ws_url(base_url: &str, path: &str) -> String {
    let http_url = normalize_url(base_url, path);
    if let Some(rest) = http_url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = http_url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        http_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_covers_seven_targets() {
        let all = BlackMagicProxyCatalog::all_specs();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_router_supports_api_and_ws() {
        let spec = BlackMagicProxyCatalog::by_target(BlackMagicProxyTarget::ClaudeCodeRouter)
            .expect("router spec should exist");
        assert!(spec
            .exposures
            .iter()
            .any(|it| it.kind == ProxyExposureKind::Api));
        assert!(spec
            .exposures
            .iter()
            .any(|it| it.kind == ProxyExposureKind::WebSocket));
    }

    #[test]
    fn test_prepare_http_call_newapi() {
        let client =
            BlackMagicProxyClient::from_target_default(BlackMagicProxyTarget::NewApi, "test-key")
                .expect("client should build");

        let req = ProxyChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![ProxyMessage::user("hello")],
            temperature: Some(0.2),
            stream: Some(false),
        };

        let call = client
            .prepare_call(ProxyExposureKind::Api, &req)
            .expect("prepare should pass");

        match call {
            ProxyPreparedCall::Http(call) => {
                assert_eq!(call.method, "POST");
                assert!(call.url.ends_with("/v1/chat/completions"));
                assert_eq!(
                    call.headers.get("authorization"),
                    Some(&"Bearer test-key".to_string())
                );
            }
            _ => panic!("should be http call"),
        }
    }

    #[test]
    fn test_prepare_ws_call_router() {
        let client = BlackMagicProxyClient::from_target_default(
            BlackMagicProxyTarget::ClaudeCodeRouter,
            "router-key",
        )
        .expect("client should build");

        let req = ProxyChatRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ProxyMessage::user("ping")],
            temperature: None,
            stream: Some(true),
        };

        let call = client
            .prepare_call(ProxyExposureKind::WebSocket, &req)
            .expect("prepare should pass");

        match call {
            ProxyPreparedCall::WebSocket(call) => {
                assert!(call.url.starts_with("ws://") || call.url.starts_with("wss://"));
                assert_eq!(
                    call.headers.get("x-api-key"),
                    Some(&"router-key".to_string())
                );
            }
            _ => panic!("should be websocket call"),
        }
    }

    #[test]
    fn test_prepare_cli_call_cliproxyapi() {
        let client = BlackMagicProxyClient::from_target_default(
            BlackMagicProxyTarget::CliProxyApi,
            "cli-token",
        )
        .expect("client should build");

        let req = ProxyChatRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ProxyMessage::user("do task")],
            temperature: None,
            stream: None,
        };

        let call = client
            .prepare_call(ProxyExposureKind::Cli, &req)
            .expect("prepare should pass");

        match call {
            ProxyPreparedCall::Cli(call) => {
                assert_eq!(call.command, "claude");
                assert!(call
                    .env
                    .get("NEUROLOOM_PROXY_TOKEN")
                    .is_some_and(|v| v == "cli-token"));
            }
            _ => panic!("should be cli call"),
        }
    }
}
