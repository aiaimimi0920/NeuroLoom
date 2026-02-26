use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability, Modality};

/// 可灵 AI (Kling) 模型解析器
///
/// 支持的模型名称参考新版可灵文档：
/// - kling-v1
/// - kling-v1-6
/// - kling-v2-master
pub struct KlingModelResolver {
    default_resolver: DefaultModelResolver,
}

impl KlingModelResolver {
    pub fn new() -> Self {
        Self {
            default_resolver: DefaultModelResolver::new(),
        }
    }
}

impl ModelResolver for KlingModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 短别名映射
        match model.to_lowercase().as_str() {
            "v1" | "kling-v1" => "kling-v1".to_string(),
            "v1.6" | "v1-6" | "kling-v1-6" => "kling-v1-6".to_string(),
            "v2" | "kling-v2" | "kling-v2-master" => "kling-v2-master".to_string(),
            
            _ => {
                // 如果用户输入了未知模型，直接传给 Kling
                model.to_string()
            }
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // 可灵模型专注于视频/图像生成，目前通过 VISION 涵盖广义视觉或直接默认 false 等
        // 由于没有设计专用的 VideoGen 标志，我们暂不开放 Text 核心能力，只保留最基础的能力
        match capability {
            Capability::VISION => true,
            _ => false,
        }
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        // 对于视频生成大模型不涉及上下文 token 数量，给予 0
        (0, 0)
    }

    fn max_context(&self, _model: &str) -> usize {
        0
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, Modality)> {
        Some((4.0, Modality::ImageGeneration))
    }
}
