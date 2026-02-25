use crate::client::ClientBuilder;
use crate::site::base::dify::DifySite;
use crate::protocol::base::dify::DifyProtocol;
use crate::provider::dify::DifyModelResolver;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(DifySite::new())
        .protocol(DifyProtocol {})
        .model_resolver(DifyModelResolver::new())
        .default_model("dify")
}
