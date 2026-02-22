use crate::auth::{TokenStatus, TokenStorage, AuthError};
use std::path::Path;

/// Gemini CLI OAuth 配置
pub const GEMINI_CLI_OAUTH_CONFIG: GeminiCliOAuthConfig = GeminiCliOAuthConfig {
    client_id: "TODO_client_id",
    client_secret: "TODO_client_secret",
    redirect_port: 8085,
    auth_url: "https://accounts.google.com/o/oauth2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    scopes: &["https://www.googleapis.com/auth/cloud-platform"],
};

#[derive(Debug, Clone)]
pub struct GeminiCliOAuthConfig {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

pub struct GeminiCliOAuth {
    #[allow(dead_code)]
    config: GeminiCliOAuthConfig,
    storage: Option<TokenStorage>,
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl GeminiCliOAuth {
    pub fn new() -> Self {
        Self {
            config: GEMINI_CLI_OAUTH_CONFIG.clone(),
            storage: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn from_file(_path: &Path) -> Result<Self, AuthError> {
        Ok(Self::new())
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
    fn test_gemini_oauth() {
        let oauth = GeminiCliOAuth::new();
        assert!(oauth.needs_refresh());
    }
}
