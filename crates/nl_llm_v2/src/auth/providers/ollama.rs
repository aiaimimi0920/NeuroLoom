use async_trait::async_trait; use reqwest::RequestBuilder; use crate::auth::Authenticator; use crate::site::context::AuthType;
pub struct OllamaAuth { api_key: String }
impl OllamaAuth { pub fn new(api_key: impl Into<String>) -> Self { let api_key = api_key.into().trim().trim_start_matches("Bearer ").trim_start_matches("bearer ").to_string(); Self { api_key } } }
#[async_trait] impl Authenticator for OllamaAuth { fn id(&self)->&str{"ollama"} fn is_authenticated(&self)->bool{true} fn inject(&self, req: RequestBuilder)->anyhow::Result<RequestBuilder>{if !self.api_key.is_empty(){Ok(req.header("Authorization", format!("Bearer {}", self.api_key)))}else{Ok(req)}} fn auth_type(&self)->AuthType{AuthType::ApiKey} }
