//! 生成参数定义

use serde::{Deserialize, Serialize};

/// 生成参数
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrimitiveParameters {
    /// 最大输出 Token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,

    /// 温度参数 (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-P 采样
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// 停止序列
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// 思考/推理配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

/// 思考配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// 是否启用思考模式
    pub enabled: bool,
    /// 思考预算 Token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u64>,
}

impl PrimitiveParameters {
    /// 创建新的参数配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大 Token 数
    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置 Top-P
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// 设置停止序列
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// 启用思考模式
    pub fn with_thinking(mut self, budget_tokens: Option<u64>) -> Self {
        self.thinking = Some(ThinkingConfig {
            enabled: true,
            budget_tokens,
        });
        self
    }
}
