//! Antigravity (Gemini Code Assist) 模型列表展示
//!
//! 由于 Cloud Code (PA) 端点没有开放的 `/models` 查询接口，
//! 此代码展示后端原生支持的 Antigravity Gemini 模型清单。

fn main() {
    println!("========================================");
    println!("  Antigravity Models (nl_llm_new)");
    println!("========================================");
    println!();
    
    println!("Google Cloud Code (PA) 支持的已知模型列表:");
    println!("----------------------------------------");
    let models = vec![
        "gemini-2.5-flash",
        "gemini-2.5-pro",
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gemini-exp-1206",
        "claude-3-5-sonnet-v2@20241022",
    ];
    
    for model in models {
        println!("  - {}", model);
    }
    println!("----------------------------------------");
}
