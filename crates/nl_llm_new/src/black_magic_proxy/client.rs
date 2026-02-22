//! 统一代理客户端

use std::collections::BTreeMap;

use super::types::*;
use super::catalog::BlackMagicProxyCatalog;

/// 错误类型
#[derive(Debug, Clone)]
pub struct ProxyError {
    pub message: String,
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProxyError {}

/// 统一代理客户端（当前阶段做"接口归一化 + 调用准备"）
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
    ) -> Result<Self, ProxyError> {
        let spec = BlackMagicProxyCatalog::by_target(target).ok_or_else(|| {
            ProxyError {
                message: "proxy target spec not found".to_string(),
            }
        })?;

        Ok(Self::new(target, spec.default_base_url, credential))
    }

    pub fn list_supported_exposures(&self) -> Result<Vec<ProxyExposure>, ProxyError> {
        let spec = BlackMagicProxyCatalog::by_target(self.target).ok_or_else(|| {
            ProxyError {
                message: "proxy target spec not found".to_string(),
            }
        })?;
        Ok(spec.exposures)
    }

    /// 按指定形态准备调用参数
    pub fn prepare_call(
        &self,
        exposure_kind: ProxyExposureKind,
        request: &ProxyChatRequest,
    ) -> Result<ProxyPreparedCall, ProxyError> {
        let spec = BlackMagicProxyCatalog::by_target(self.target).ok_or_else(|| {
            ProxyError {
                message: "proxy target spec not found".to_string(),
            }
        })?;

        let exposure = spec
            .exposures
            .iter()
            .find(|it| it.kind == exposure_kind)
            .ok_or_else(|| {
                ProxyError {
                    message: format!(
                        "exposure kind {:?} not supported for target {:?}",
                        exposure_kind, self.target
                    ),
                }
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
    ) -> Result<ProxyPreparedHttpCall, ProxyError> {
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
            ProxyError {
                message: format!("serialize request failed: {e}"),
            }
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
    ) -> Result<ProxyPreparedWsCall, ProxyError> {
        let mut headers = BTreeMap::new();
        if let Some(header) = &exposure.auth_header {
            let prefix = exposure.auth_prefix.clone().unwrap_or_default();
            headers.insert(
                header.to_ascii_lowercase(),
                format!("{}{}", prefix, self.credential),
            );
        }

        let init_payload = serde_json::to_value(request).map_err(|e| {
            ProxyError {
                message: format!("serialize request failed: {e}"),
            }
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
    ) -> Result<ProxyPreparedCliCall, ProxyError> {
        let command = exposure
            .cli_command
            .clone()
            .ok_or_else(|| ProxyError {
                message: "cli command missing".to_string(),
            })?;

        let input_payload = serde_json::to_string(request).map_err(|e| {
            ProxyError {
                message: format!("serialize request failed: {e}"),
            }
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
