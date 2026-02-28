//! 并发许可证（RAII 模式）

use std::time::Instant;
use tokio::sync::SemaphorePermit;

use super::controller::{ConcurrencyController, FailureType};

/// 许可证，持有期间占用一个并发槽位
///
/// 使用 RAII 模式，drop 时自动释放槽位。
pub struct ConcurrencyPermit<'a> {
    controller: &'a ConcurrencyController,
    start_time: Instant,
    /// permit 通过 drop 自动释放，不需要手动读取
    #[allow(dead_code)]
    permit: Option<SemaphorePermit<'a>>,
    reported: bool,
}

impl<'a> ConcurrencyPermit<'a> {
    /// 创建新的许可证
    pub(super) fn new(controller: &'a ConcurrencyController, permit: SemaphorePermit<'a>) -> Self {
        Self {
            controller,
            start_time: Instant::now(),
            permit: Some(permit),
            reported: false,
        }
    }

    /// 手动报告成功（自动计算延迟）
    pub fn report_success(mut self) {
        let latency = self.start_time.elapsed().as_millis() as u64;
        self.controller.report_success(latency);
        self.reported = true;
        // permit 自动释放
    }

    /// 手动报告失败
    pub fn report_failure(mut self, error_type: FailureType) {
        self.controller.report_failure(error_type);
        self.reported = true;
        // permit 自动释放
    }

    /// 获取已用时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

impl Drop for ConcurrencyPermit<'_> {
    fn drop(&mut self) {
        // 释放活跃请求计数
        self.controller.release_active();

        // 如果没有手动报告，不做任何统计
        // permit 通过 drop 自动释放
    }
}
