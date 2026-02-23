use std::collections::HashMap;
use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::{UrlContext, Action, AuthType};

/// Google Cloud Vertex AI 网关
pub struct VertexSite {
    project_id: String,
    location: String,
    timeout: Duration,
}

impl VertexSite {
    /// 创建 VertexSite
    /// 注意：project_id 和 location 应该从 Service Account JSON 中获取或手动设置
    pub fn new(project_id: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            location: location.into(),
            timeout: Duration::from_secs(120),
        }
    }

    /// [新增] 设置 project_id
    /// 原因：允许在构建客户端时动态设置 project_id
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = project_id.into();
        self
    }

    /// [新增] 设置 location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = location.into();
        self
    }

    /// [新增] 设置超时
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Site for VertexSite {
    fn id(&self) -> &str {
        "vertex"
    }

    fn base_url(&self) -> &str {
        // Base Url 未必适用，因为这里结构太特殊了
        "https://aiplatform.googleapis.com"
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let action_suffix = match ctx.action {
            Action::Generate => "generateContent",
            Action::Stream => "streamGenerateContent?alt=sse",
            _ => "generateContent",
        };

        match ctx.auth_type {
            AuthType::ServiceAccount => {
                format!(
                    "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:{}",
                    self.location, self.project_id, self.location, ctx.model, action_suffix
                )
            },
            AuthType::ApiKey | _ => {
                format!(
                    "https://us-central1-aiplatform.googleapis.com/v1beta1/projects/{}/locations/us-central1/publishers/google/models/{}:{}",
                    self.project_id, ctx.model, action_suffix
                )
            }
        }
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
