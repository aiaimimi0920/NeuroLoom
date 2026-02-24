use crate::pipeline::traits::PipelineContext;
use crate::protocol::traits::ProtocolHook;
use serde_json::Value;
use uuid::Uuid;

/// 注入 CloudCode 必需的 Project 参数和 Headers
pub struct CloudCodeHook;

impl CloudCodeHook {
    /// 自动生成一个随机项目用于绕过
    pub fn generate_project_id() -> String {
        let uid = Uuid::new_v4().to_string();
        format!("project-{}", &uid[..6])
    }
}

impl ProtocolHook for CloudCodeHook {
    fn after_pack(&self, ctx: &mut PipelineContext<'_>, packed: &mut Value) {
        // CloudCode API 需要的 payload 结构非标准 Gemini，需要在外部包装一层
        // 取出原有的 payload (通常是 {"contents": [...], "systemInstruction": ...})
        let original_request = packed.clone();

        let model = match &ctx.input {
            crate::pipeline::traits::PipelineInput::Primitive(p) => p.model.clone(),
            _ => "gemini-2.5-flash".to_string(),
        };

        let request_id = format!("agent-{}", Uuid::new_v4());
        
        let project_val = ctx.auth_extra.get("project_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(""); // 必须提供 project 字段，支持空字符串跳过计费校验

        let wrapper = serde_json::json!({
            "model": model,
            "userAgent": "cloud-code-protocol",
            "requestType": "agent",
            "project": project_val,
            "requestId": request_id,
            "request": original_request
        });

        *packed = wrapper;

        // 插入 sessionId 以符合 CloudCode 规范 (注意: 必须为纯负数字符串，否则将由于后端整数越界或解析异常而触发 500 INTERNAL_ERROR)
        if let Some(req_obj) = packed.get_mut("request").and_then(|r| r.as_object_mut()) {
            if !req_obj.contains_key("sessionId") {
                let d = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                req_obj.insert(
                    "sessionId".to_string(),
                    serde_json::Value::String(format!("-{}", d & 0x7FFFFFFFFFFFFFFF)),
                );
            }
        }
    }

    // [修复] 移除 before_send 实现
    // 原因：RequestBuilder 的 try_clone() 可能失败，且在 hook 中修改不可靠
    // Headers 应该通过 Site::extra_headers() 或专门的 Header 注入机制处理
    // 当前 CloudCode 所需 Headers 可以在 CloudCodeSite 中配置
}
