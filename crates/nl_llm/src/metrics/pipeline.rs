//! Pipeline 执行指标

use std::collections::HashMap;
use std::time::Instant;

/// Pipeline 执行指标
#[derive(Debug, Clone)]
pub struct PipelineMetrics {
    /// 请求开始时间
    pub start_time: Instant,

    /// 请求结束时间
    pub end_time: Option<Instant>,

    /// 总响应时间（毫秒）
    pub response_time_ms: Option<u64>,

    /// 首个 Token 时间（流式请求）
    pub first_token_time_ms: Option<u64>,

    /// 各阶段耗时（毫秒）
    pub stage_timings: HashMap<String, u64>,

    /// 是否成功
    pub success: bool,

    /// 错误信息（如果失败）
    pub error_message: Option<String>,
}

impl PipelineMetrics {
    /// 创建新的指标记录
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            end_time: None,
            response_time_ms: None,
            first_token_time_ms: None,
            stage_timings: HashMap::new(),
            success: true,
            error_message: None,
        }
    }

    /// 记录阶段耗时
    pub fn record_stage(&mut self, stage: &str, duration_ms: u64) {
        self.stage_timings.insert(stage.to_string(), duration_ms);
    }

    /// 记录首个 Token 时间
    pub fn record_first_token(&mut self) {
        if self.first_token_time_ms.is_none() {
            self.first_token_time_ms = Some(self.start_time.elapsed().as_millis() as u64);
        }
    }

    /// 标记为成功并完成记录
    pub fn finish_success(mut self) -> Self {
        self.end_time = Some(Instant::now());
        self.response_time_ms = Some(self.start_time.elapsed().as_millis() as u64);
        self.success = true;
        self
    }

    /// 标记为失败并完成记录
    pub fn finish_error(mut self, error: &str) -> Self {
        self.end_time = Some(Instant::now());
        self.response_time_ms = Some(self.start_time.elapsed().as_millis() as u64);
        self.success = false;
        self.error_message = Some(error.to_string());
        self
    }

    /// 完成记录（不改变成功状态）
    pub fn finish(&mut self) {
        self.end_time = Some(Instant::now());
        self.response_time_ms = Some(self.start_time.elapsed().as_millis() as u64);
    }

    /// 获取总耗时（毫秒）
    pub fn total_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

impl Default for PipelineMetrics {
    fn default() -> Self {
        Self::new()
    }
}
