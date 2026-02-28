/// 生成参数
#[derive(Debug, Clone, Default)]
pub struct PrimitiveParameters {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Vec<String>,
}
