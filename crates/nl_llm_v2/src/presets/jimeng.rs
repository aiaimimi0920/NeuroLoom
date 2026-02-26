use crate::client::ClientBuilder;
use crate::model::jimeng::JimengModelResolver;
use crate::provider::jimeng::JimengExtension;
use crate::site::base::jimeng::JimengSite;

pub fn default_preset() -> ClientBuilder {
    ClientBuilder::new()
        .site(JimengSite::new())
        .model_resolver(JimengModelResolver::new())
        .with_extension(std::sync::Arc::new(JimengExtension::new()))
        .protocol(crate::protocol::base::openai::OpenAiProtocol)
        .default_model("jimeng-video")
}
