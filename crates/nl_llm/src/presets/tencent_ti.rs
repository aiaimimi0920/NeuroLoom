use crate::client::ClientBuilder;
use crate::protocol::base::tencent_ti::TencentTiProtocol;
use crate::provider::hunyuan::{HunyuanHook, HunyuanModelResolver};
use crate::site::base::tencent_ti::TencentTiSite;
use std::sync::Arc;

pub fn builder() -> ClientBuilder {
    let hook = Arc::new(HunyuanHook {});
    ClientBuilder::new()
        .site(TencentTiSite::new())
        .protocol(TencentTiProtocol {})
        .with_extension(hook)
        .model_resolver(HunyuanModelResolver::new())
        .default_model("hunyuan-turbos-latest")
}
