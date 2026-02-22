//! 代理目录（用于文档、配置 UI、诊断）

use super::types::*;

/// 黑魔法代理目录
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
                        path: "/v1internal/request".to_string(),
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
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::Vertex,
                default_base_url: "https://us-central1-aiplatform.googleapis.com".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/projects/{project}/locations/us-central1/publishers/google/models/{model}:generateContent".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("Authorization".to_string()),
                        auth_prefix: Some("Bearer ".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Vertex AI Gemini 非流式生成接口（SA JSON 认证）".to_string(),
                    },
                ],
                notes: "Google Cloud Vertex AI Gemini 接口（仅 Service Account JSON 认证）".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::GoogleAIStudio,
                default_base_url: "https://generativelanguage.googleapis.com".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1beta/models/{model}:generateContent".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("x-goog-api-key".to_string()),
                        auth_prefix: Some("".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Google AI Studio Gemini API（API Key 认证）".to_string(),
                    },
                ],
                notes: "Google AI Studio (generativelanguage.googleapis.com)".to_string(),
            },
            BlackMagicProxySpec {
                target: BlackMagicProxyTarget::VertexCompat,
                default_base_url: "".to_string(),
                exposures: vec![
                    ProxyExposure {
                        kind: ProxyExposureKind::Api,
                        path: "/v1/publishers/google/models/{model}:generateContent".to_string(),
                        method: "POST".to_string(),
                        auth_header: Some("x-goog-api-key".to_string()),
                        auth_prefix: Some("".to_string()),
                        cli_command: None,
                        cli_args: vec![],
                        notes: "Vertex Compat 第三方转发站接口".to_string(),
                    },
                ],
                notes: "Vertex Compat Provider（第三方转发站如 zenmux.ai）".to_string(),
            },
        ]
    }

    pub fn by_target(target: BlackMagicProxyTarget) -> Option<BlackMagicProxySpec> {
        Self::all_specs().into_iter().find(|it| it.target == target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_covers_ten_targets() {
        let all = BlackMagicProxyCatalog::all_specs();
        assert_eq!(all.len(), 10);
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
}
