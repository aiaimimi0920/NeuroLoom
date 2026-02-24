//! openai 平台测试 - models
//!
//! 本例展示 OpenAiModelResolver 对各模型的 alias 和 capability 解析

use nl_llm_v2::model::resolver::Capability;

fn main() {
    println!("========================================");
    println!("  OpenAI API Models Test");
    println!("========================================\n");

    // 使用 OpenAI 预设的 OpenAiModelResolver
    let client = nl_llm_v2::presets::openai::builder()
        .with_api_key("dummy_key")
        .build();

    let test_models = vec![
        "gpt-4o",
        "gpt-4-turbo",
        "gpt3", // Alias for gpt-3.5-turbo
        "o1",
        "o1-mini",
        "o3-mini",
        "unknown-model",
    ];

    for model in test_models {
        let resolved = client.resolve_model(model);
        let (ctx, _) = client.context_window_hint(model);
        
        let mut caps = Vec::new();
        if client.has_capability(model, Capability::CHAT) { caps.push("CHAT"); }
        if client.has_capability(model, Capability::VISION) { caps.push("VISION"); }
        if client.has_capability(model, Capability::TOOLS) { caps.push("TOOLS"); }
        if client.has_capability(model, Capability::STREAMING) { caps.push("STREAMING"); }
        if client.has_capability(model, Capability::THINKING) { caps.push("THINKING"); }

        println!("Alias '{}' -> Resolved: '{}'", model, resolved);
        println!("  Context Limits: {}", ctx);
        println!("  Capabilities: [{}]", caps.join(", "));
        println!();
    }
}
