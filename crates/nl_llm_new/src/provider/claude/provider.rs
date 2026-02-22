use super::{config::{ClaudeConfig, ClaudeAuth}, compiler::ClaudeCompiler};
use crate::auth::providers::claude::ClaudeOAuth;
use crate::auth::Auth;
use crate::provider::{LlmProvider, LlmResponse};
use crate::primitive::PrimitiveRequest;
use async_trait::async_trait;
use crate::provider::BoxStream;
use crate::provider::LlmChunk;

pub struct ClaudeProvider {
    config: ClaudeConfig,
    compiler: ClaudeCompiler,
    #[allow(dead_code)]
    oauth: Option<ClaudeOAuth>,
    #[allow(dead_code)]
    http: reqwest::Client,
    /// Cached Auth enum for trait method
    auth_enum: Auth,
}

impl ClaudeProvider {
    pub fn new(config: ClaudeConfig) -> Self {
        let auth_enum = match &config.auth {
            ClaudeAuth::ApiKey(cfg) => Auth::ApiKey(cfg.clone()),
            ClaudeAuth::OAuth { token_path } => Auth::OAuth {
                provider: crate::auth::OAuthProvider::Claude,
                token_path: token_path.clone(),
            },
        };

        let oauth = match &config.auth {
            ClaudeAuth::OAuth { token_path } => ClaudeOAuth::from_file(token_path).ok(),
            _ => None,
        };

        Self {
            config,
            compiler: ClaudeCompiler,
            oauth,
            http: reqwest::Client::new(),
            auth_enum,
        }
    }

    pub fn is_official(&self) -> bool {
        match &self.config.auth {
            ClaudeAuth::ApiKey(cfg) => cfg.base_url.is_none(),
            ClaudeAuth::OAuth { .. } => true,
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &["claude-sonnet-4-5-20250929", "claude-opus-4-5-20251101", "claude-3-5-haiku-20241022"]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        self.compiler.compile(primitive)
    }

    async fn complete(&self, _body: serde_json::Value) -> crate::Result<LlmResponse> {
        unimplemented!("TDD: implement real HTTP execution")
    }

    async fn stream(&self, _body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        unimplemented!("TDD: implement stream")
    }

    fn needs_refresh(&self) -> bool {
        self.oauth.as_ref().map_or(false, |o| o.needs_refresh())
    }

    async fn refresh_auth(&mut self) -> crate::Result<()> {
        if let Some(oauth) = &mut self.oauth {
            oauth.refresh_token().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        }
        Ok(())
    }
}
