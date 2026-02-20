//! Prompt AST 与方言编译器
//!
//! 对齐 `docs/架构.md` 的模型无关防腐层设计：
//! - 认知层输入使用 AST，而不是字符串拼接
//! - 再按 Provider 方言进行编译

use serde::{Deserialize, Serialize};

/// Prompt AST 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptNode {
    System(String),
    User(String),
    Assistant(String),
    Tool { name: String, content: String },
}

/// Prompt AST 容器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptAst {
    nodes: Vec<PromptNode>,
}

impl PromptAst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, node: PromptNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn nodes(&self) -> &[PromptNode] {
        &self.nodes
    }

    /// Anthropic 友好的 XML 组织（简化版）
    pub fn to_anthropic_xml(&self) -> String {
        let mut out = String::from("<prompt>");
        for node in &self.nodes {
            match node {
                PromptNode::System(v) => {
                    out.push_str("<system>");
                    out.push_str(&xml_escape(v));
                    out.push_str("</system>");
                }
                PromptNode::User(v) => {
                    out.push_str("<user>");
                    out.push_str(&xml_escape(v));
                    out.push_str("</user>");
                }
                PromptNode::Assistant(v) => {
                    out.push_str("<assistant>");
                    out.push_str(&xml_escape(v));
                    out.push_str("</assistant>");
                }
                PromptNode::Tool { name, content } => {
                    out.push_str("<tool name=\"");
                    out.push_str(&xml_escape(name));
                    out.push_str("\">");
                    out.push_str(&xml_escape(content));
                    out.push_str("</tool>");
                }
            }
        }
        out.push_str("</prompt>");
        out
    }

    /// OpenAI 兼容 messages JSON（简化版）
    pub fn to_openai_messages(&self) -> Vec<serde_json::Value> {
        self.nodes
            .iter()
            .map(|node| match node {
                PromptNode::System(v) => serde_json::json!({"role":"system","content":v}),
                PromptNode::User(v) => serde_json::json!({"role":"user","content":v}),
                PromptNode::Assistant(v) => {
                    serde_json::json!({"role":"assistant","content":v})
                }
                PromptNode::Tool { name, content } => serde_json::json!({
                    "role":"tool",
                    "name":name,
                    "content":content
                }),
            })
            .collect()
    }

    /// Ollama 友好的 ChatML 扁平化（简化版）
    pub fn to_chatml(&self) -> String {
        let mut lines = Vec::new();
        for node in &self.nodes {
            match node {
                PromptNode::System(v) => lines.push(format!("<|system|>\n{v}")),
                PromptNode::User(v) => lines.push(format!("<|user|>\n{v}")),
                PromptNode::Assistant(v) => lines.push(format!("<|assistant|>\n{v}")),
                PromptNode::Tool { name, content } => {
                    lines.push(format!("<|tool:{name}|>\n{content}"))
                }
            }
        }
        lines.join("\n")
    }
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_compile_variants() {
        let ast = PromptAst::new()
            .push(PromptNode::System("你是助手".to_string()))
            .push(PromptNode::User("你好".to_string()));

        assert!(ast.to_anthropic_xml().contains("<system>"));
        assert_eq!(ast.to_openai_messages().len(), 2);
        assert!(ast.to_chatml().contains("<|user|>"));
    }
}
