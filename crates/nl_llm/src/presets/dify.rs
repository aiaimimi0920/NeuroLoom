use crate::client::ClientBuilder;
use crate::protocol::base::dify::DifyProtocol;
use crate::provider::dify::DifyModelResolver;
use crate::site::base::dify::DifySite;

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(DifySite::new())
        .protocol(DifyProtocol {})
        .model_resolver(DifyModelResolver::new())
        .default_model("dify")
}
