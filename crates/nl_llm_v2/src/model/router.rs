use crate::model::resolver::{Capability, Modality};

/// 任务路由模式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingMode {
    /// 速度优先：选择 TTFT 最短或 TPS 最高的组合
    SpeedFirst,
    /// 价格优先：根据有效成本估算，选择总价最低的组合
    PriceFirst,
    /// 准确/智能优先：选择最新实际智能等级（扣除渠道惩罚后）最高的模型
    AccuracyFirst,
}

/// 安全与隐私等级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityClearance {
    /// 廉价不信任的公网代理池，可能落库数据
    L1UntrustedProxy = 1,
    /// 官方或受信任的云服务商（不用于训练）
    L2TrustedCloud = 2,
    /// 局域网内物理隔离的本地部署模型
    L3LocalPrivate = 3,
}

/// 路由强制约束条件
pub struct RouteConstraint {
    /// 需要调用的模型功能模态
    pub modality: Modality,
    /// 所需的最低实际智能等级（基础分数减去惩罚分数后）
    pub min_intelligence: f32,
    /// 该次生成所预估的总上下文长度 (输入+预估输出)
    pub estimated_tokens: usize,
    /// 任务所需的最低安全隐私等级要求
    pub require_clearance: SecurityClearance,
    /// 强制要求支持的器官与特性（如 TOOLS, STREAMING）
    pub require_capabilities: Capability,
}

/// 统一的运行平台候选者（结合了 Provider 与 Model）
#[derive(Debug, Clone)]
pub struct RouteCandidate {
    /// 中标的提供商 ID (Site ID / Preset ID)
    pub provider_id: String,
    /// 实际采用的基础模型名称
    pub model_name: String,
    /// 该节点实际结算的智能积分 (包含任何中转商降智惩罚)
    pub actual_intelligence: f32, 
    /// 混合评估成本 (可能会依赖认证方式，0元党最高优)
    pub cost_estimate: f32,       
    /// 标定速度 (首字延迟 TTFT 毫秒, 每秒生成 token TPS)
    pub speed_metrics: (u64, u64),
}

/// 核心路由调度器
///
/// 决定一条 `Prompt AST` 到底飞往哪个渠道的最优模型节点
pub trait Router: Send + Sync {
    /// 在不指定确切模型的前提下，根据需求图谱挑选最佳候选者
    fn route(
        &self, 
        mode: RoutingMode, 
        constraint: RouteConstraint,
    ) -> anyhow::Result<RouteCandidate>;
    
    /// 反馈回路监控
    /// 当下游执行器遭遇某个渠道由于 429/500 限流崩溃时的回调。
    /// Router 将基于此在内存中将其置为 Cooldown 状态，防止并发雪崩。
    fn report_failure(&self, provider_id: &str, model_name: &str);
}
