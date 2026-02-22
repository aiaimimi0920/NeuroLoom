//! Token 持久化工具
//!
//! 提供统一的 Token 存储和加载功能

use std::path::Path;

use super::{AuthError, TokenStorage};

/// Token 持久化存储
pub struct TokenStorageManager;

impl TokenStorageManager {
    /// 保存 Token 到文件
    pub fn save<P: AsRef<Path>>(storage: &TokenStorage, path: P) -> Result<(), AuthError> {
        let path = path.as_ref();

        // 创建父目录
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 序列化并写入
        let content = serde_json::to_string_pretty(storage)?;
        std::fs::write(path, content)?;

        tracing::debug!("Token saved to {:?}", path);
        Ok(())
    }

    /// 从文件加载 Token
    pub fn load<P: AsRef<Path>>(path: P) -> Result<TokenStorage, AuthError> {
        let path = path.as_ref();

        let content = std::fs::read_to_string(path)?;
        let storage: TokenStorage = serde_json::from_str(&content)?;

        tracing::debug!("Token loaded from {:?}", path);
        Ok(storage)
    }

    /// 检查文件是否存在
    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    /// 删除 Token 文件
    pub fn delete<P: AsRef<Path>>(path: P) -> Result<(), AuthError> {
        let path = path.as_ref();
        if path.exists() {
            std::fs::remove_file(path)?;
            tracing::debug!("Token file deleted: {:?}", path);
        }
        Ok(())
    }
}

/// 生成 Token 文件名
pub fn token_filename(provider: &str, identifier: &str) -> String {
    format!("{}-{}.json", provider, identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_token_storage_roundtrip() {
        let storage = TokenStorage::new("test_access_token", "claude")
            .with_refresh_token("test_refresh_token")
            .with_email("test@example.com")
            .with_expires_at(Utc::now() + Duration::hours(1));

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_token_storage.json");

        // 保存
        TokenStorageManager::save(&storage, &path).unwrap();

        // 加载
        let loaded = TokenStorageManager::load(&path).unwrap();

        assert_eq!(loaded.access_token, storage.access_token);
        assert_eq!(loaded.refresh_token, storage.refresh_token);
        assert_eq!(loaded.email, storage.email);
        assert_eq!(loaded.provider, storage.provider);

        // 清理
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_token_status() {
        let valid = TokenStorage::new("token", "test")
            .with_expires_at(Utc::now() + Duration::hours(1));
        assert_eq!(valid.status(300), crate::auth::TokenStatus::Valid);

        let expiring = TokenStorage::new("token", "test")
            .with_expires_at(Utc::now() + Duration::seconds(100));
        assert_eq!(expiring.status(300), crate::auth::TokenStatus::ExpiringSoon);

        let expired = TokenStorage::new("token", "test")
            .with_expires_at(Utc::now() - Duration::seconds(10));
        assert_eq!(expired.status(300), crate::auth::TokenStatus::Expired);
    }
}
