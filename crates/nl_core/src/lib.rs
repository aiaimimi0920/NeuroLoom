//! # nl_core - NeuroLoom Core Primitives
//!
//! 核心原语层，定义事件溯源事件枚举、UUID、全局错误处理机制。
//! 此 crate 是整个项目的基础依赖，不依赖其他业务 crate。

pub mod error;
pub mod event;
pub mod entity;

pub use error::{NeuroLoomError, Result};
pub use event::{Event, EventKind};
pub use entity::{Entity, EntityId};
