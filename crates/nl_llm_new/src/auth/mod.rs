//! 认证层
//!
//! 提供三种认证类型的统一抽象：
//! - OAuth：需要浏览器登录，Token 会过期
//! - API Key：直接使用，不过期
//! - Service Account：JSON 凭据，JWT 认证

mod types;
mod storage;

pub mod providers;

pub use types::*;
pub use storage::*;
