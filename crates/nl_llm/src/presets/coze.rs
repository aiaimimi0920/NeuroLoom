use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::coze::CozeProtocol;
use crate::provider::coze::CozeExtension;
use crate::site::base::coze::CozeSite;
use std::sync::Arc;

/// Coze 默认配置
pub fn default_builder() -> ClientBuilder {
    LlmClient::builder()
        .site(CozeSite::new())
        .protocol(CozeProtocol {})
        .with_extension(Arc::new(CozeExtension::new()))
        .default_model("coze-bot-id") // User needs to overwrite this with the actual bot ID
}
