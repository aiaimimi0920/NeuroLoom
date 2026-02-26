use async_trait::async_trait; use reqwest::RequestBuilder; use crate::auth::Authenticator; use crate::site::context::AuthType;
pub struct XfyunMaasAuth { api_key: String }
impl XfyunMaasAuth { pub fn new(api_key: impl Into<String>) -> Self { let api_key = api_key.into().trim().trim_start_matches("Bearer ").trim_start_matches("bearer ").to_string(); Self { api_key } } }
#[async_trait] impl Authenticator for XfyunMaasAuth { fn id(&self)->&str{"xfyun_maas"} fn is_authenticated(&self)->bool{!self.api_key.is_empty()} fn inject(&self, req: RequestBuilder)->anyhow::Result<RequestBuilder>{Ok(req.header("Authorization", format!("Bearer {}", self.api_key)))} fn auth_type(&self)->AuthType{AuthType::ApiKey} }
