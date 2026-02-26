use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::auth::Authenticator;
use crate::pipeline::traits::{PipelineContext, Stage};

/// 认证阶段：注入认证上下文并判断是否需要执行刷新
pub struct AuthenticateStage {
    authenticator: Arc<Mutex<Box<dyn Authenticator>>>,
}

impl AuthenticateStage {
    pub fn new(authenticator: Arc<Mutex<Box<dyn Authenticator>>>) -> Self {
        Self { authenticator }
    }
}

#[async_trait]
impl Stage for AuthenticateStage {
    fn name(&self) -> &str {
        "authenticate"
    }

    async fn process(&self, ctx: &mut PipelineContext<'_>) -> anyhow::Result<()> {
        let mut auth = self.authenticator.lock().await;

        if auth.needs_refresh() {
            auth.refresh().await?;
        }

        // 更新 Context 信息以备后续发送阶段 URL 构建之用
        ctx.url_context.auth_type = auth.auth_type();

        if let Some(extras) = auth.get_extra() {
            ctx.auth_extra = extras.clone();
        }

        Ok(())
    }
}
