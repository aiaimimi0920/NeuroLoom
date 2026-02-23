use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use reqwest::Client;

use crate::pipeline::traits::{Stage, PipelineContext, PipelineInput};
use crate::auth::Authenticator;
use crate::site::Site;
use crate::protocol::traits::{ProtocolFormat, ProtocolHook};

/// 发送阶段：构建 reqwest 实体并最终发送
pub struct SendStage {
    site: Arc<dyn Site>,
    authenticator: Arc<Mutex<Box<dyn Authenticator>>>,
    http: Client,
    /// [新增] 协议引用，用于错误规范化
    protocol: Arc<dyn ProtocolFormat>,
    /// [新增] 协议钩子，用于响应预处理
    hooks: Vec<Arc<dyn ProtocolHook>>,
}

impl SendStage {
    pub fn new(
        site: Arc<dyn Site>,
        authenticator: Arc<Mutex<Box<dyn Authenticator>>>,
        http: Client,
        protocol: Arc<dyn ProtocolFormat>,
    ) -> Self {
        Self { site, authenticator, http, protocol, hooks: Vec::new() }
    }

    /// [新增] 带钩子的构造函数
    pub fn with_hooks(
        site: Arc<dyn Site>,
        authenticator: Arc<Mutex<Box<dyn Authenticator>>>,
        http: Client,
        protocol: Arc<dyn ProtocolFormat>,
        hooks: Vec<Arc<dyn ProtocolHook>>,
    ) -> Self {
        Self { site, authenticator, http, protocol, hooks }
    }
}

#[async_trait]
impl Stage for SendStage {
    fn name(&self) -> &str {
        "send"
    }

    async fn process(&self, ctx: &mut PipelineContext<'_>) -> anyhow::Result<()> {
        if let PipelineInput::Packed(data) = &ctx.input {
            let url = self.site.build_url(&ctx.url_context);

            let mut req = self.http.post(&url)
                .json(data)
                .timeout(self.site.timeout());

            for (k, v) in self.site.extra_headers() {
                req = req.header(k, v);
            }

            // [新增] 调用 before_send 钩子
            // 原因：允许 hook 在发送前修改请求（如添加特殊 headers）
            for hook in &self.hooks {
                hook.before_send(ctx, &mut req);
            }

            let auth = self.authenticator.lock().await;
            req = auth.inject(req)?;

            // 发起请求
            let mut resp = req.send().await?;

            // [修复] 使用 protocol.unpack_error 规范化错误处理
            // 原因：错误需要包含 retryable 和 fallback_hint 信息，便于上层决策
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let raw = resp.text().await.unwrap_or_default();
                let standard_error = self.protocol.unpack_error(status, &raw)?;
                return Err(anyhow::anyhow!("{}", standard_error));
            }

            // [新增] 调用 after_receive 钩子
            // 原因：允许 hook 在接收响应后进行预处理（如处理特殊 headers）
            for hook in &self.hooks {
                hook.after_receive(ctx, &mut resp);
            }

            // [修复] 区分流式和非流式响应处理
            // 原因：流式请求需要保存 Response 对象供 unpack_stream 使用
            let is_stream = ctx.url_context.action == crate::site::context::Action::Stream;

            if is_stream {
                // 流式：保存 Response 对象，由 UnpackStage 处理
                ctx.input = PipelineInput::RawResponse(resp);
            } else {
                // 非流式：读取 bytes 供解包
                let bytes = resp.bytes().await?.to_vec();
                ctx.input = PipelineInput::Raw(bytes);
            }
        }

        Ok(())
    }
}
