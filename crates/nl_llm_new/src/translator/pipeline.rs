//! 转换管道

use serde_json::Value;

use super::{detect_wrapper, error::TranslateError, Format, WrapperKind};
use crate::primitive::PrimitiveRequest;

/// 转换管道
pub struct TranslatorPipeline {
    source_format: Format,
    target_format: Format,
}

impl TranslatorPipeline {
    /// 创建新的转换管道
    pub fn new(source: Format, target: Format) -> Self {
        Self {
            source_format: source,
            target_format: target,
        }
    }

    /// 转换请求
    pub fn translate(&self, input: &[u8]) -> Result<Vec<u8>, TranslateError> {
        // 解析输入
        let parsed: Value = serde_json::from_slice(input)?;

        // 快速路径：相同格式直通
        if self.source_format == self.target_format {
            return Ok(input.to_vec());
        }

        // 检测包裹
        let wrapper = detect_wrapper(&parsed);

        // 检查是否可以直通
        if self.can_passthrough(wrapper) {
            return Ok(input.to_vec());
        }

        // 完整转换流程：解包 → 原语 → 封装
        let primitive = self.unwrap(&parsed, wrapper)?;
        let output = self.wrap(&primitive)?;

        Ok(serde_json::to_vec(&output)?)
    }

    /// 是否可以直通
    fn can_passthrough(&self, wrapper: WrapperKind) -> bool {
        matches!(
            (wrapper, self.source_format, self.target_format),
            (WrapperKind::ClaudeCode, Format::Claude, Format::Claude)
                | (WrapperKind::GeminiCLI, Format::GeminiCLI, Format::GeminiCLI)
                | (WrapperKind::None, Format::OpenAI, Format::OpenAI)
        )
    }

    /// 解包为原语
    fn unwrap(
        &self,
        parsed: &Value,
        wrapper: WrapperKind,
    ) -> Result<PrimitiveRequest, TranslateError> {
        // 根据 source_format 选择对应的 unwrapper
        match self.source_format {
            Format::Claude => super::unwrapper::claude::unwrap(parsed, wrapper),
            Format::OpenAI => super::unwrapper::openai::unwrap(parsed, wrapper),
            Format::Gemini => super::unwrapper::gemini::unwrap(parsed, wrapper),
            Format::GeminiCLI => super::unwrapper::gemini_cli::unwrap(parsed, wrapper),
            Format::Codex => super::unwrapper::codex::unwrap(parsed, wrapper),
            Format::Antigravity => super::unwrapper::antigravity::unwrap(parsed, wrapper),
            Format::OpenAIResponse => {
                super::unwrapper::openai::unwrap_response(parsed, wrapper)
            }
        }
    }

    /// 封装为目标格式
    fn wrap(&self, primitive: &PrimitiveRequest) -> Result<Value, TranslateError> {
        // 根据 target_format 选择对应的 wrapper
        match self.target_format {
            Format::Claude => super::wrapper::claude::wrap(primitive),
            Format::OpenAI => super::wrapper::openai::wrap(primitive),
            Format::Gemini => super::wrapper::gemini::wrap(primitive),
            Format::GeminiCLI => super::wrapper::gemini_cli::wrap(primitive),
            Format::Codex => super::wrapper::codex::wrap(primitive),
            Format::Antigravity => super::wrapper::antigravity::wrap(primitive),
            Format::OpenAIResponse => super::wrapper::openai::wrap_response(primitive),
        }
    }
}
