use bitflags::bitflags;

bitflags! {
    /// 模型能力标志
    ///
    /// 用于描述模型支持的功能特性，方便上层根据能力选择合适的模型。
    ///
    /// # 示例
    ///
    /// ```
    /// use nl_llm::model::Capability;
    ///
    /// let caps = Capability::CHAT | Capability::VISION | Capability::STREAMING;
    ///
    /// assert!(caps.contains(Capability::CHAT));
    /// assert!(caps.contains(Capability::VISION));
    /// assert!(!caps.contains(Capability::TOOLS));
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Capability: u32 {
        /// 基础对话能力
        /// 所有模型都应具备此能力
        const CHAT = 0b0001;

        /// 视觉理解能力
        /// 支持图像输入和理解
        const VISION = 0b0010;

        /// 工具调用能力
        /// 支持 function calling / tool use
        const TOOLS = 0b0100;

        /// 流式输出能力
        /// 支持 SSE ���式响应
        const STREAMING = 0b1000;

        /// 深度思考能力
        /// 支持 thinking/reasoning 模式
        const THINKING = 0b10000;

        /// 代码执行能力
        /// 支持内置代码执行环境
        const CODE_EXECUTION = 0b100000;
    }
}

/// 模型解析器
///
/// 负责模型别名解析和能力检测，为上层提供模型相关的元信息查询。
///
/// # 职责
///
/// 1. **别名解析**: 将用户友好的别名转换为实际模型名
///    - `"gpt4"` → `"gpt-4o"`
///    - `"claude"` → `"claude-sonnet-4-20250514"`
///
/// 2. **能力检测**: 判断模型是否支持特定功能
///    - 用于选择合适的模型执行任务
///
/// 3. **上下文窗口**: 提供上下文长度限制和建议分配
///
/// # 实现说明
///
/// 平台级 ModelResolver 通常委托给 `DefaultModelResolver`，
/// 仅覆盖特定平台的别名和能力配置。
///
/// # 示例
///
/// ```
/// use nl_llm::model::{ModelResolver, Capability};
/// use nl_llm::model::DefaultModelResolver;
///
/// let resolver = DefaultModelResolver::new();
///
/// // 别名解析
/// assert_eq!(resolver.resolve("gpt4"), "gpt-4o");
///
/// // 能力检测
/// assert!(resolver.has_capability("gpt-4o", Capability::VISION));
///
/// // 上下文窗口
/// let (input_limit, output_limit) = resolver.context_window_hint("gpt-4o");
/// println!("Input: {}, Output: {}", input_limit, output_limit);
/// ```
pub trait ModelResolver: Send + Sync {
    /// 解析模型别名到实际模型名
    ///
    /// 如果传入的名称不是别名，则原样返回。
    ///
    /// # 示例
    ///
    /// ```
    /// // 别名
    /// assert_eq!(resolver.resolve("gpt4"), "gpt-4o");
    ///
    /// // 非别名
    /// assert_eq!(resolver.resolve("gpt-4o"), "gpt-4o");
    /// ```
    fn resolve(&self, model: &str) -> String;

    /// 检查模型是否支持指定能力
    ///
    /// 会先解析别名，再检查能力。
    ///
    /// # 示例
    ///
    /// ```
    /// assert!(resolver.has_capability("gpt4", Capability::VISION));
    /// assert!(!resolver.has_capability("o1", Capability::STREAMING));
    /// ```
    fn has_capability(&self, model: &str, cap: Capability) -> bool;

    /// 获取模型的最大上下文长度
    ///
    /// 返回 token 数量上限。如果模型未知，返回默认值 4096。
    ///
    /// # 示例
    ///
    /// ```
    /// let max = resolver.max_context("gpt-4o");
    /// assert_eq!(max, 128_000);
    /// ```
    fn max_context(&self, model: &str) -> usize;

    fn context_window_hint(&self, model: &str) -> (usize, usize);

    /// 获取模型的基础智能评级与模态分类
    ///
    /// 返回 (智能等级 1.0~5.0, 所属模态)
    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, Modality)>;
}

/// 模型模态分类
///
/// 决定了模型能处理的数据类型或擅长的具体领域
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modality {
    /// 纯文本模型，专精语言逻辑
    Text,
    /// 视觉理解模型 (Image-to-Text)
    Vision,
    /// 语音识别与生成模型
    Audio,
    /// 全模态大融合模型
    Multimodal,
    /// 向量化模型
    Embedding,
    /// 图像生成模型 (Text-to-Image)
    ImageGeneration,
}
