use std::sync::Arc;
use crate::client::{ClientBuilder, LlmClient};
use crate::site::base::coze::CozeSite;
use crate::protocol::base::coze::CozeProtocol;
use crate::provider::coze::CozeExtension;

/// Coze 默认配置
pub fn default_builder() -> ClientBuilder {
    LlmClient::builder()
        .site(CozeSite::new())
        .protocol(CozeProtocol {})
        .with_extension(Arc::new(CozeExtension::new()))
        .default_model("coze-bot-id") // User needs to overwrite this with the actual bot ID
}
