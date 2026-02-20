//! # nl_cognitive - NeuroLoom Cognitive Engine
//!
//! 双擎认知引擎：System 1 (SOP 固化) + System 2 (MCTS 自适应)。
//! 包含 Worker、Critic、MoA 议会的裁判逻辑。

pub mod system1;
pub mod system2;
pub mod courtroom;
pub mod blacksmith;

pub use system1::SopEngine;
pub use system2::MctsEngine;
pub use courtroom::{Courtroom, Verdict};
pub use blacksmith::Blacksmith;
