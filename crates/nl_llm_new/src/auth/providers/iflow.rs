use crate::auth::{TokenStatus, TokenStorage, AuthError};

/// IFlow (Cookie -> Token) 认证
pub struct IFlowAuth {
    storage: Option<TokenStorage>,
}

impl IFlowAuth {
    pub fn new() -> Self {
        Self { storage: None }
    }

    pub fn from_cookie(_cookie: &str) -> Result<Self, AuthError> {
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
    fn test_iflow_auth() {
        let auth = IFlowAuth::new();
        assert!(auth.needs_refresh());
    }
}
