use async_trait::async_trait;
use rand::Rng;
use reqwest::RequestBuilder;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

use crate::auth::Authenticator;
use crate::site::context::AuthType;

#[derive(Debug, Clone, Copy)]
pub enum MultiKeyMode {
    Random,
    RoundRobin,
}

/// 多 Key 认证器（用于 new-api 这类代理平台）
///
/// - 支持随机与轮询两种选 key 策略
/// - 支持在运行时临时禁用故障 key
pub struct MultiKeyAuth {
    keys: Vec<String>,
    mode: MultiKeyMode,
    cursor: AtomicUsize,
    disabled: RwLock<HashSet<usize>>,
    header_name: String,
    is_bearer: bool,
}

impl MultiKeyAuth {
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            keys,
            mode: MultiKeyMode::RoundRobin,
            cursor: AtomicUsize::new(0),
            disabled: RwLock::new(HashSet::new()),
            header_name: "Authorization".to_string(),
            is_bearer: true,
        }
    }

    pub fn with_mode(mut self, mode: MultiKeyMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_header(mut self, header_name: impl Into<String>, is_bearer: bool) -> Self {
        self.header_name = header_name.into();
        self.is_bearer = is_bearer;
        self
    }

    pub fn disable_key(&self, idx: usize) {
        if idx < self.keys.len() {
            if let Ok(mut disabled) = self.disabled.write() {
                disabled.insert(idx);
            }
        }
    }

    pub fn enable_key(&self, idx: usize) {
        if let Ok(mut disabled) = self.disabled.write() {
            disabled.remove(&idx);
        }
    }

    fn pick_index(&self) -> Option<usize> {
        if self.keys.is_empty() {
            return None;
        }

        let disabled = self.disabled.read().ok()?;
        let candidates: Vec<usize> = (0..self.keys.len())
            .filter(|idx| !disabled.contains(idx))
            .collect();
        drop(disabled);

        if candidates.is_empty() {
            return None;
        }

        match self.mode {
            MultiKeyMode::Random => {
                let pick = rand::thread_rng().gen_range(0..candidates.len());
                Some(candidates[pick])
            }
            MultiKeyMode::RoundRobin => {
                let step = self.cursor.fetch_add(1, Ordering::Relaxed);
                Some(candidates[step % candidates.len()])
            }
        }
    }
}

#[async_trait]
impl Authenticator for MultiKeyAuth {
    fn id(&self) -> &str {
        "multi_key"
    }

    fn is_authenticated(&self) -> bool {
        !self.keys.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        let idx = self
            .pick_index()
            .ok_or_else(|| anyhow::anyhow!("没有可用的 API key（可能全部被禁用）"))?;

        let key = self
            .keys
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("无效的 key 索引: {}", idx))?;

        let val = if self.is_bearer {
            format!("Bearer {}", key)
        } else {
            key.clone()
        };
        Ok(req.header(&self.header_name, val))
    }

    fn auth_type(&self) -> AuthType {
        AuthType::MultiKey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_works() {
        let auth = MultiKeyAuth::new(vec!["k1".into(), "k2".into(), "k3".into()]);
        assert_eq!(auth.pick_index(), Some(0));
        assert_eq!(auth.pick_index(), Some(1));
        assert_eq!(auth.pick_index(), Some(2));
        assert_eq!(auth.pick_index(), Some(0));
    }

    #[test]
    fn disable_key_filters_candidates() {
        let auth = MultiKeyAuth::new(vec!["k1".into(), "k2".into()]);
        auth.disable_key(0);
        assert_eq!(auth.pick_index(), Some(1));
    }
}
