use crate::auth::{TokenStatus, TokenStorage, AuthError};
use std::path::Path;

/// Claude OAuth 配置（Claude 特有常量）
pub const CLAUDE_OAUTH_CONFIG: ClaudeOAuthConfig = ClaudeOAuthConfig {
    client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
    redirect_port: 54545,
    auth_url: "https://claude.ai/oauth/authorize",
    token_url: "https://console.anthropic.com/v1/oauth/token",
    scopes: &[
        "org:create_api_key",
        "user:profile",
        "user:inference",
    ],
};

/// Claude OAuth 配置
#[derive(Debug, Clone)]
pub struct ClaudeOAuthConfig {
    pub client_id: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

/// Claude PKCE 挑战码
#[derive(Debug, Clone)]
pub struct ClaudePkceCodes {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// Claude OAuth 客户端
pub struct ClaudeOAuth {
    #[allow(dead_code)]
    config: ClaudeOAuthConfig,
    #[allow(dead_code)]
    storage: Option<TokenStorage>,
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl ClaudeOAuth {
    pub fn new() -> Self {
        Self {
            config: CLAUDE_OAUTH_CONFIG.clone(),
            storage: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn from_file(_path: &Path) -> Result<Self, AuthError> {
        // TDD TODO: implement loading from token_path
        Ok(Self::new())
    }

    pub fn generate_pkce(&self) -> ClaudePkceCodes {
        ClaudePkceCodes {
            code_verifier: "TODO_verifier".to_string(),
            code_challenge: "TODO_challenge".to_string(),
        }
    }

    pub fn build_auth_url(&self, _state: &str, _pkce: &ClaudePkceCodes) -> String {
        "TODO_auth_url".to_string()
    }

    pub async fn exchange_code(
        &mut self,
        _code: &str,
        _pkce: &ClaudePkceCodes
    ) -> Result<(), AuthError> {
        Ok(())
    }

    pub async fn refresh_token(&mut self) -> Result<(), AuthError> {
        Ok(())
    }

    pub fn access_token(&self) -> Option<&str> {
        self.storage.as_ref().map(|s| s.access_token.as_str())
    }

    pub fn needs_refresh(&self) -> bool {
        self.storage.as_ref().map_or(true, |s| {
            matches!(s.status(300), TokenStatus::Expired | TokenStatus::ExpiringSoon)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_oauth_init() {
        let oauth = ClaudeOAuth::new();
        assert!(oauth.needs_refresh());
        assert_eq!(oauth.access_token(), None);
    }
}
