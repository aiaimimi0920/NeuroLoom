/// iFlow Provider 配置
#[derive(Debug, Clone)]
pub struct IFlowConfig {
    /// Cookie 用于获取 API Key
    pub cookie: String,
    /// 默认模型
    pub model: String,
    /// Token 缓存文件路径
    pub token_path: std::path::PathBuf,
}

impl IFlowConfig {
    /// 创建新的配置
    pub fn new(cookie: String, model: String, token_path: std::path::PathBuf) -> Self {
        Self {
            cookie,
            model,
            token_path,
        }
    }

    /// 使用默认 token 路径创建配置
    pub fn with_default_path(cookie: String, model: String) -> Self {
        let token_path = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("neuroloom")
            .join("iflow_token.json");

        Self {
            cookie,
            model,
            token_path,
        }
    }
}
