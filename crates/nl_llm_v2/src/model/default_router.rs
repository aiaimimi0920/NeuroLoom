use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::model::resolver::{Capability, Modality};
use crate::model::router::{
    RouteCandidate, RouteConstraint, Router, RoutingMode, SecurityClearance,
};

/// 节点冷却记录
#[derive(Debug, Clone)]
struct CooldownRecord {
    until: Instant,
    failures: usize,
}

/// 一个在注册表中等待被路由分配的可用物理节点 (Endpoint)
#[derive(Debug, Clone)]
pub struct RouterEndpoint {
    pub provider_id: String,
    pub model_name: String,

    // 基础智能
    pub base_intelligence: f32,
    // 造假与中转降智惩罚
    pub penalty: f32,

    pub modality: Modality,
    pub max_context: usize,
    pub capabilities: Capability,
    pub clearance: SecurityClearance,

    // 是否官方信誉良好 (影响 Accuracy 兜底排序)
    pub is_official_or_trusted: bool,

    // 动态计价估算 / 动态速度估算
    // 在真实应用中，这可能会通过一个探测器定期更新
    pub current_cost_estimate: f32,
    pub ttft_ms: u64,
    pub tps: u64,
}

impl RouterEndpoint {
    pub fn actual_intelligence(&self) -> f32 {
        self.base_intelligence - self.penalty
    }
}

/// NeuroLoom 默认的智能路由调度器
pub struct DefaultRouter {
    endpoints: RwLock<Vec<RouterEndpoint>>,
    cooldowns: RwLock<HashMap<String, CooldownRecord>>,
    // 缺省的冷却时间
    default_cooldown_duration: Duration,
}

impl DefaultRouter {
    pub fn new() -> Self {
        Self {
            endpoints: RwLock::new(Vec::new()),
            cooldowns: RwLock::new(HashMap::new()),
            default_cooldown_duration: Duration::from_secs(300), // 默认封印五分钟
        }
    }

    /// 注册一个新的可用端点入池
    pub fn register_endpoint(&self, endpoint: RouterEndpoint) {
        let mut eps = self.endpoints.write().unwrap();
        eps.push(endpoint);
    }

    /// 清理过期的冷却记录
    fn clean_expired_cooldowns(&self) {
        let mut cds = self.cooldowns.write().unwrap();
        let now = Instant::now();
        cds.retain(|_, record| record.until > now);
    }
}

impl Router for DefaultRouter {
    fn route(
        &self,
        mode: RoutingMode,
        constraint: RouteConstraint,
    ) -> anyhow::Result<RouteCandidate> {
        self.clean_expired_cooldowns();

        let eps = self.endpoints.read().unwrap();
        let cds = self.cooldowns.read().unwrap();

        // 预选池过滤
        let mut candidates: Vec<&RouterEndpoint> = eps
            .iter()
            .filter(|ep| {
                // 过滤冷却节点
                let key = format!("{}:{}", ep.provider_id, ep.model_name);
                if cds.contains_key(&key) {
                    return false;
                }

                // 1. 模态过滤 （Multimodal 比较特殊）
                if constraint.modality != ep.modality {
                    // 如果任务只要求 Text，但是节点是 Multimodal，这是可以当兼容平替的
                    // 只是稍后的计价可能会让它排在后头
                    if constraint.modality == Modality::Text && ep.modality == Modality::Multimodal
                    {
                        // 放行兼容
                    } else {
                        return false;
                    }
                }

                // 2. 实际智能度下限
                if ep.actual_intelligence() < constraint.min_intelligence {
                    return false;
                }

                // 3. 上下文长度限制
                if ep.max_context < constraint.estimated_tokens {
                    return false;
                }

                // 4. 安全等级约束
                if ep.clearance < constraint.require_clearance {
                    return false;
                }

                // 5. 特性能力约束
                if !ep.capabilities.contains(constraint.require_capabilities) {
                    return false;
                }

                true
            })
            .collect();

        if candidates.is_empty() {
            return Err(anyhow::anyhow!(
                "No suitable endpoints found in orchestration pool"
            ));
        }

        // 执行排序逻辑
        match mode {
            RoutingMode::AccuracyFirst => {
                // 智能度最高 -> 信誉好 -> TTFT最快 -> 价格最低
                candidates.sort_by(|a, b| {
                    b.actual_intelligence()
                        .partial_cmp(&a.actual_intelligence())
                        .unwrap()
                        .then_with(|| b.is_official_or_trusted.cmp(&a.is_official_or_trusted))
                        .then_with(|| a.ttft_ms.cmp(&b.ttft_ms))
                        .then_with(|| {
                            a.current_cost_estimate
                                .partial_cmp(&b.current_cost_estimate)
                                .unwrap()
                        })
                });
            }
            RoutingMode::SpeedFirst => {
                // TTFT最快 -> TPS最大 -> 智能度最高 -> 价格极差
                candidates.sort_by(|a, b| {
                    a.ttft_ms
                        .cmp(&b.ttft_ms)
                        .then_with(|| b.tps.cmp(&a.tps))
                        .then_with(|| {
                            b.actual_intelligence()
                                .partial_cmp(&a.actual_intelligence())
                                .unwrap()
                        })
                });
            }
            RoutingMode::PriceFirst => {
                // 价格最低 -> 智能最高
                candidates.sort_by(|a, b| {
                    a.current_cost_estimate
                        .partial_cmp(&b.current_cost_estimate)
                        .unwrap()
                        .then_with(|| {
                            b.actual_intelligence()
                                .partial_cmp(&a.actual_intelligence())
                                .unwrap()
                        })
                });
            }
        }

        let best = candidates[0];
        Ok(RouteCandidate {
            provider_id: best.provider_id.clone(),
            model_name: best.model_name.clone(),
            actual_intelligence: best.actual_intelligence(),
            cost_estimate: best.current_cost_estimate,
            speed_metrics: (best.ttft_ms, best.tps),
        })
    }

    fn report_failure(&self, provider_id: &str, model_name: &str) {
        let key = format!("{}:{}", provider_id, model_name);
        let mut cds = self.cooldowns.write().unwrap();

        let record = cds.entry(key).or_insert_with(|| CooldownRecord {
            until: Instant::now() + self.default_cooldown_duration,
            failures: 0,
        });

        record.failures += 1;
        record.until = Instant::now() + self.default_cooldown_duration; // 刷新封印时间
    }
}

// ==========================================================
// 单元测试
// ==========================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_router() -> DefaultRouter {
        let router = DefaultRouter::new();

        // 节点A: 高端官方模型 (Claude 3.5 Sonnet)
        router.register_endpoint(RouterEndpoint {
            provider_id: "anthropic".into(),
            model_name: "claude-3.5-sonnet".into(),
            base_intelligence: 5.0,
            penalty: 0.0,
            modality: Modality::Multimodal,
            max_context: 200_000,
            capabilities: Capability::CHAT | Capability::TOOLS | Capability::VISION,
            clearance: SecurityClearance::L2TrustedCloud,
            is_official_or_trusted: true,
            current_cost_estimate: 15.0, // $$
            ttft_ms: 1200,
            tps: 60,
        });

        // 节点B: 廉价中转池子提供的 "Claude 3.5 Sonnet" (实际上可能掺水 Qwen)
        router.register_endpoint(RouterEndpoint {
            provider_id: "cheap_proxy".into(),
            model_name: "claude-3.5-sonnet".into(),
            base_intelligence: 5.0,
            penalty: 3.0, // 虽然标称 L5，但被惩罚 3级，实际结算 2.0
            modality: Modality::Multimodal,
            max_context: 200_000,
            capabilities: Capability::CHAT | Capability::TOOLS | Capability::VISION,
            clearance: SecurityClearance::L1UntrustedProxy,
            is_official_or_trusted: false,
            current_cost_estimate: 0.5, // 极度便宜
            ttft_ms: 3000,
            tps: 20,
        });

        // 节点C: 官方便宜快模型 (Gemini Flash)
        router.register_endpoint(RouterEndpoint {
            provider_id: "google".into(),
            model_name: "gemini-flash".into(),
            base_intelligence: 3.5,
            penalty: 0.0,
            modality: Modality::Multimodal,
            max_context: 1_000_000,
            capabilities: Capability::CHAT | Capability::TOOLS | Capability::VISION,
            clearance: SecurityClearance::L2TrustedCloud,
            is_official_or_trusted: true,
            current_cost_estimate: 1.0,
            ttft_ms: 400, // 极速
            tps: 120,     // 极速
        });

        // 节点D: 本地部署大模型 (Local Llama)
        router.register_endpoint(RouterEndpoint {
            provider_id: "local_ollama".into(),
            model_name: "llama-3-70b".into(),
            base_intelligence: 4.0,
            penalty: 0.0,
            modality: Modality::Text, // 纯文本
            max_context: 32_000,
            capabilities: Capability::CHAT | Capability::TOOLS,
            clearance: SecurityClearance::L3LocalPrivate, // 最高安全级别
            is_official_or_trusted: true,
            current_cost_estimate: 0.0, // 电费忽略不计，视为免费
            ttft_ms: 5000,              // 非常慢
            tps: 15,
        });

        router
    }

    #[test]
    fn test_accuracy_first_reputation_fallback() {
        let router = build_test_router();
        // 增加一个和 Claude 官方同样智商，但非官方，且便宜的节点
        router.register_endpoint(RouterEndpoint {
            provider_id: "another_ok_proxy".into(),
            model_name: "claude-3.5-sonnet".into(),
            base_intelligence: 5.0,
            penalty: 0.0, // 满级不掺水
            modality: Modality::Multimodal,
            max_context: 200_000,
            capabilities: Capability::CHAT | Capability::TOOLS,
            clearance: SecurityClearance::L1UntrustedProxy,
            is_official_or_trusted: false,
            current_cost_estimate: 8.0,
            ttft_ms: 1000,
            tps: 50,
        });

        let constraint = RouteConstraint {
            modality: Modality::Text,
            min_intelligence: 4.5,
            estimated_tokens: 10_000,
            require_clearance: SecurityClearance::L1UntrustedProxy,
            require_capabilities: Capability::CHAT,
        };

        // AccuracyFirst，当官方节点和代理节点都是 5.0 的真实智商时，官方节点胜出 (虽然代理节点更便宜)
        let best = router
            .route(RoutingMode::AccuracyFirst, constraint)
            .unwrap();
        assert_eq!(best.provider_id, "anthropic");
    }

    #[test]
    fn test_fraud_penalty_filtration() {
        let router = build_test_router();
        let constraint = RouteConstraint {
            modality: Modality::Text,
            min_intelligence: 4.0, // 要求至少 4.0
            estimated_tokens: 1000,
            require_clearance: SecurityClearance::L1UntrustedProxy,
            require_capabilities: Capability::CHAT,
        };

        // cheap_proxy 虽然标称 claude-3.5-sonnet，但扣除 3 分后跌至 2.0，达不到 4.0 的要求，被淘汰
        // anthropic (5.0) 会胜出
        let best = router
            .route(RoutingMode::AccuracyFirst, constraint)
            .unwrap();
        assert_eq!(best.provider_id, "anthropic");
    }

    #[test]
    fn test_price_first_free_tier() {
        let router = build_test_router();
        let constraint = RouteConstraint {
            modality: Modality::Text,
            min_intelligence: 1.0, // 只要能跑就行
            estimated_tokens: 1000,
            require_clearance: SecurityClearance::L1UntrustedProxy,
            require_capabilities: Capability::CHAT,
        };

        // PriceFirst 会选本地节点，因为它 cost 是 0.0
        let best = router.route(RoutingMode::PriceFirst, constraint).unwrap();
        assert_eq!(best.provider_id, "local_ollama");
    }

    #[test]
    fn test_speed_first() {
        let router = build_test_router();
        let constraint = RouteConstraint {
            modality: Modality::Multimodal, // 指定需要眼睛
            min_intelligence: 2.0,
            estimated_tokens: 100,
            require_clearance: SecurityClearance::L1UntrustedProxy,
            require_capabilities: Capability::VISION,
        };

        // SpeedFirst 应该直接挑落所有的老慢牛，精准命中 Gemini Flash
        let best = router.route(RoutingMode::SpeedFirst, constraint).unwrap();
        assert_eq!(best.provider_id, "google");
    }

    #[test]
    fn test_security_clearance_constraint() {
        let router = build_test_router();
        let constraint = RouteConstraint {
            modality: Modality::Text,
            min_intelligence: 3.0,
            estimated_tokens: 100,
            require_clearance: SecurityClearance::L3LocalPrivate, // 最高机密
            require_capabilities: Capability::CHAT,
        };

        let best = router
            .route(RoutingMode::AccuracyFirst, constraint)
            .unwrap();
        // 只有 L3 级别的 local_ollama 能接这个活
        assert_eq!(best.provider_id, "local_ollama");
    }

    #[test]
    fn test_circuit_breaker() {
        let router = build_test_router();
        let constraint = RouteConstraint {
            modality: Modality::Multimodal,
            min_intelligence: 3.0,
            estimated_tokens: 100,
            require_clearance: SecurityClearance::L2TrustedCloud,
            require_capabilities: Capability::CHAT,
        };

        let best1 = router
            .route(RoutingMode::SpeedFirst, constraint.clone())
            .unwrap();
        assert_eq!(best1.provider_id, "google"); // 快人一步

        // 模拟 Google 发生 429 崩溃熔断
        router.report_failure("google", "gemini-flash");

        // 再次请求
        let best2 = router.route(RoutingMode::SpeedFirst, constraint).unwrap();
        assert_eq!(best2.provider_id, "anthropic"); // 自动降级至备选
    }
}
