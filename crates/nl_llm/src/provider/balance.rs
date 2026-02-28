//! 余额与额度状态类型定义
//!
//! 本模块定义了各平台余额查询的返回类型，供上层调度器做决策。
//!
//! # 设计原则
//!
//! - Provider 层只负责"提供信息"
//! - 决策逻辑（如何时降优先级、暂停调用）由上层处理

use chrono::{DateTime, Utc};

/// 计费单位
#[derive(Debug, Clone)]
pub enum BillingUnit {
    /// Token 数量（最常见）
    Tokens,
    /// 请求次数 (RPD/RPM)
    Requests,
    /// 金额（美元、人民币等）
    Money { currency: String },
}

/// 额度状态
///
/// 描述单一额度（免费或付费）的使用情况。
#[derive(Debug, Clone)]
pub struct QuotaStatus {
    /// 计费单位
    pub unit: BillingUnit,

    /// 已使用量
    pub used: f64,

    /// 总量限制（None = 无限制）
    pub total: Option<f64>,

    /// 剩余量（None = 未知或无限制）
    pub remaining: Option<f64>,

    /// 剩余比例 (0.0-1.0)，未知则为 None
    ///
    /// - 1.0 = 满额
    /// - 0.0 = 耗尽
    pub remaining_ratio: Option<f32>,

    /// 是否会自动重置（如每日/每月）
    pub resets: bool,

    /// 重置时间
    pub reset_at: Option<DateTime<Utc>>,
}

/// 额度类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaType {
    /// 仅免费额度
    FreeOnly,
    /// 仅付费余额
    PaidOnly,
    /// 混合（有免费也有付费）
    Mixed,
    /// 未知/不支持查询
    Unknown,
}

/// 余额状态（整体）
///
/// 包含平台余额的完整信息，供上层调度器决策。
///
/// # 使用示例
///
/// ```rust
/// use nl_llm::provider::balance::{BalanceStatus, QuotaType, QuotaStatus, BillingUnit};
///
/// let status = BalanceStatus {
///     display: "免费额度: 800/1000 tokens".to_string(),
///     quota_type: QuotaType::FreeOnly,
///     free: Some(QuotaStatus {
///         unit: BillingUnit::Tokens,
///         used: 200.0,
///         total: Some(1000.0),
///         remaining: Some(800.0),
///         remaining_ratio: Some(0.8),
///         resets: true,
///         reset_at: None,
///     }),
///     paid: None,
///     has_free_quota: true,
///     should_deprioritize: false,
///     is_unavailable: false,
/// };
///
/// // 上层调度器可以据此决策
/// if status.has_free_quota {
///     println!("可以继续调用免费额度");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BalanceStatus {
    /// 可读描述（用于日志/显示）
    ///
    /// 例如："免费额度: 800/1000 tokens" 或 "余额: $12.34"
    pub display: String,

    /// 额度类型
    pub quota_type: QuotaType,

    /// 免费额度状态（如果有）
    pub free: Option<QuotaStatus>,

    /// 付费余额状态（如果有）
    pub paid: Option<QuotaStatus>,

    /// 是否还有可用免费额度
    ///
    /// 便捷字段，等价于 `free.as_ref().map(|q| q.remaining.unwrap_or(0.0) > 0.0).unwrap_or(false)`
    pub has_free_quota: bool,

    /// 是否应该降低优先级
    ///
    /// 由各平台实现决定阈值逻辑，如：
    /// - DeepSeek: 赠送余额 < 10% 时返回 true
    /// - Gemini: RPD 接近上限时返回 true
    pub should_deprioritize: bool,

    /// 是否完全不可用
    ///
    /// - API 错误导致无法查询
    /// - 余额完全耗尽且无重置机制
    pub is_unavailable: bool,
}

impl BalanceStatus {
    /// 创建一个不支持余额查询的状态
    pub fn unsupported() -> Self {
        Self {
            display: "不支持余额查询".to_string(),
            quota_type: QuotaType::Unknown,
            free: None,
            paid: None,
            has_free_quota: false,
            should_deprioritize: false,
            is_unavailable: false,
        }
    }

    /// 创建一个查询失败的状态
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            display: message.into(),
            quota_type: QuotaType::Unknown,
            free: None,
            paid: None,
            has_free_quota: false,
            should_deprioritize: false,
            is_unavailable: true,
        }
    }
}
