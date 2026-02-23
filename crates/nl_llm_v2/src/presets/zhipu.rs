use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://open.bigmodel.cn/api/paas/v4"))
        .protocol(OpenAiProtocol {})
        .default_model("glm-4")
}
