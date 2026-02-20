//! 视觉流处理

use crate::delta_diff::SemanticDiff;

/// 视觉流
pub struct VisionStream {
    /// 差分器
    diff: SemanticDiff,
    /// 是否运行中
    running: bool,
}

impl VisionStream {
    /// 创建新视觉流
    pub fn new() -> Self {
        Self {
            diff: SemanticDiff::default(),
            running: false,
        }
    }

    /// 启动流
    pub fn start(&mut self) {
        self.running = true;
    }

    /// 停止流
    pub fn stop(&mut self) {
        self.running = false;
        self.diff.reset();
    }

    /// 处理帧
    pub fn process_frame(&mut self, frame: &[u8]) -> crate::delta_diff::FrameDiff {
        self.diff.process(frame)
    }

    /// 是否运行中
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for VisionStream {
    fn default() -> Self {
        Self::new()
    }
}
