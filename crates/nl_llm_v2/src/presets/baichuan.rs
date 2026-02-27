use crate::client::ClientBuilder;
use crate::model::baichuan::BaichuanModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::SimpleSite;
use std::collections::HashMap;
use std::time::Duration;

/// 万变不离其宗的 Baichuan 预设
/// 百川 API 完全兼容 OpenAI 的 /v1/chat/completions 接口
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(SimpleSite {
            id: "baichuan".to_string(),
            base_url: "https://api.baichuan-ai.com/v1".to_string(),
            extra_headers: HashMap::new(),
            timeout: Duration::from_secs(120),
        })
        .protocol(OpenAiProtocol {})
        .model_resolver(BaichuanModelResolver::new())
        // 默认模型设为百川低成本极速版
        .default_model("Baichuan4-Air")
}
