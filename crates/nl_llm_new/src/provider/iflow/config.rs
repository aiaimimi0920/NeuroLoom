/// iFlow Provider 配置 (Cookie Based)
#[derive(Debug, Clone)]
pub struct IFlowConfig {
    pub cookie: String,
    pub model: String,
}

impl IFlowConfig {
    pub fn new(cookie: String, model: String) -> Self {
        Self { cookie, model }
    }
}
