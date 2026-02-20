//! # nl_vision - NeuroLoom Vision Stream
//!
//! 视觉流处理：语义帧差分感知器、OCR、防止显存爆炸。

pub mod delta_diff;
pub mod ocr;
pub mod stream;

pub use delta_diff::SemanticDiff;
pub use stream::VisionStream;
