use crate::client::ClientBuilder;
use crate::site::base::jimeng::JimengSite;
use crate::model::jimeng::JimengModelResolver;
use crate::provider::jimeng::JimengExtension;

pub fn default_preset() -> ClientBuilder {
    ClientBuilder::new()
        .site(JimengSite::new())
        .model_resolver(JimengModelResolver::new())
        .with_extension(std::sync::Arc::new(JimengExtension::new()))
}
