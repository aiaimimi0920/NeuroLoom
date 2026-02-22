//! 格式和包裹检测

use serde_json::Value;

use super::{Format, WrapperKind};

/// 检测输入是否带有包裹
pub fn detect_wrapper(request: &Value) -> WrapperKind {
    // Claude Code 特征：system 以特定身份开头
    if let Some(system) = request.get("system").and_then(|s| s.as_array()) {
        if system
            .first()
            .and_then(|s| s.get("text"))
            .and_then(|t| t.as_str())
            .map(|t| t.contains("Claude Code"))
            .unwrap_or(false)
        {
            return WrapperKind::ClaudeCode;
        }
    }

    // Gemini CLI 特征：工具名前缀或特有工具
    if let Some(tools) = request.get("tools").and_then(|t| t.as_array()) {
        if tools.iter().any(|t| {
            t.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.starts_with("proxy_") || is_gemini_cli_builtin_tool(n))
                .unwrap_or(false)
        }) {
            return WrapperKind::GeminiCLI;
        }
    }

    // Codex 特征
    if request.get("instructions").is_some() && request.get("previous_response_id").is_some() {
        return WrapperKind::Codex;
    }

    // Antigravity 特征
    if let Some(system) = request.get("systemInstruction") {
        if contains_antigravity_signature(system) {
            return WrapperKind::Antigravity;
        }
    }

    WrapperKind::None
}

/// 检测请求格式
pub fn detect_format(request: &Value) -> Format {
    // Claude 特征：有 system 数组或使用 Claude 特有字段
    if request.get("system").is_some() && request.get("system").unwrap().is_array() {
        return Format::Claude;
    }

    // Gemini 特征：有 contents 或 systemInstruction
    if request.get("contents").is_some() || request.get("systemInstruction").is_some() {
        // 检查是否是 Gemini CLI 格式
        if detect_wrapper(request) == WrapperKind::GeminiCLI {
            return Format::GeminiCLI;
        }
        // 检查是否是 Antigravity 格式
        if detect_wrapper(request) == WrapperKind::Antigravity {
            return Format::Antigravity;
        }
        return Format::Gemini;
    }

    // Codex 特征
    if request.get("instructions").is_some() {
        return Format::Codex;
    }

    // OpenAI Response 特征
    if request.get("previous_response_id").is_some() {
        return Format::OpenAIResponse;
    }

    // 默认 OpenAI 格式
    Format::OpenAI
}

/// 检查是否为 Gemini CLI 内置工具
fn is_gemini_cli_builtin_tool(name: &str) -> bool {
    super::wrapper::GEMINI_CLI_BUILTIN_TOOLS.contains(&name)
}

/// 检查是否包含 Antigravity 签名
fn contains_antigravity_signature(system: &Value) -> bool {
    if let Some(parts) = system.get("parts").and_then(|p| p.as_array()) {
        return parts.iter().any(|part| {
            part.get("text")
                .and_then(|t| t.as_str())
                .map(|t| t.contains("Antigravity"))
                .unwrap_or(false)
        });
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_claude_code_wrapper() {
        let request = json!({
            "system": [
                {"type": "text", "text": "You are Claude Code, Anthropic's official CLI for Claude."}
            ],
            "messages": []
        });

        assert_eq!(detect_wrapper(&request), WrapperKind::ClaudeCode);
    }

    #[test]
    fn test_detect_gemini_cli_wrapper() {
        let request = json!({
            "tools": [
                {"name": "proxy_read_file"}
            ]
        });

        assert_eq!(detect_wrapper(&request), WrapperKind::GeminiCLI);
    }

    #[test]
    fn test_detect_codex_wrapper() {
        let request = json!({
            "instructions": "You are helpful",
            "previous_response_id": "resp_123"
        });

        assert_eq!(detect_wrapper(&request), WrapperKind::Codex);
    }

    #[test]
    fn test_detect_format() {
        let claude = json!({"system": [{"type": "text", "text": "test"}], "messages": []});
        assert_eq!(detect_format(&claude), Format::Claude);

        let gemini = json!({"contents": []});
        assert_eq!(detect_format(&gemini), Format::Gemini);

        let openai = json!({"messages": []});
        assert_eq!(detect_format(&openai), Format::OpenAI);
    }
}
