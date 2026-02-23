use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStorage {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub email: Option<String>,
    pub provider: String,
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenStatus {
    Valid,
    ExpiringSoon,
    Expired,
}

impl TokenStorage {
    pub fn status(&self, buffer_secs: i64) -> TokenStatus {
        match self.expires_at {
            Some(exp) => {
                let now = chrono::Utc::now();
                let diff = exp.signed_duration_since(now).num_seconds();
                if diff <= 0 {
                    TokenStatus::Expired
                } else if diff < buffer_secs {
                    TokenStatus::ExpiringSoon
                } else {
                    TokenStatus::Valid
                }
            }
            None => TokenStatus::Valid,
        }
    }
}
