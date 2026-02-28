import os

PLATFORM_INFO = {
    "openai": {"model": "gpt-4o"},
    "gemini": {"model": "gemini-2.5-flash"},
    "deepseek": {"model": "deepseek-chat"},
    "anthropic": {"model": "claude-3-5-sonnet-20241022"},
    "iflow": {"model": "qwen3-max"},
    "vertex": {"model": "gemini-2.5-flash"},
    "moonshot": {"model": "moonshot-v1-8k"},
    "zhipu": {"model": "glm-4-plus"},
    "openrouter": {"model": "google/gemini-2.5-pro"},
    "gemini_cli": {"model": "gemini-2.5-pro"},
    "antigravity": {"model": "gemini-2.5-pro"},
}

for root, dirs, files in os.walk("examples"):
    path_parts = root.replace("\\", "/").split("/")
    if len(path_parts) >= 3:
        platform = path_parts[1]
        feature = path_parts[2]
        
        info = PLATFORM_INFO.get(platform, {"model": "unknown"})
        upper_p = platform.upper()
        
        # Inject correct auth builder based on platform
        auth_inject = ".with_api_key(api_key)"
        if platform == "iflow":
            auth_inject = ".with_cookie(api_key)"
        elif platform == "vertex":
            auth_inject = ".with_service_account_json(api_key)"
        elif platform in ["gemini_cli", "antigravity"]:
            auth_inject = "" # OAuth handles itself
            
        auth_var_name = f"{upper_p}_API_KEY"
        if platform == "iflow": auth_var_name = "IFLOW_COOKIE"
        if platform == "vertex": auth_var_name = "GOOGLE_APPLICATION_CREDENTIALS_JSON"

        main_rs = f"""//! {platform} 平台测试 - {feature}
//!
//! 运行方式: cargo run --example {platform}_{feature}
//! 或直接运行: test.bat

use nl_llm::{{LlmClient, PrimitiveRequest}};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {{
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("{auth_var_name}").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("{platform}")
        .expect("Preset should exist")
        {auth_inject}
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello!".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("{info['model']}");
"""
        if feature == "stream":
            main_rs += "    req.stream = true;\n"
            main_rs += f"""
    println!("用户: {{}}\\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;
    use tokio_stream::StreamExt;
    while let Some(chunk) = stream.next().await {{
        if let Ok(c) = chunk {{
            print!("{{}}", c.content);
        }}
    }}
    println!();
    Ok(())
}}
"""
        elif feature == "tools":
            main_rs += f"""
    use serde_json::json;
    req.tools = vec![nl_llm::primitive::tool::PrimitiveTool {{
        name: "get_weather".to_string(),
        description: Some("Get current weather".to_string()),
        input_schema: json!({{"type": "object", "properties": {{"location": {{"type": "string"}}}}}}),
    }}];
    println!("用户: {{}}\\n", prompt);
    println!("AI (Tools):");

    let resp = client.complete(&req).await?;
    println!("{{:?}}", resp.content);

    Ok(())
}}
"""
        elif feature == "thinking":
            main_rs += f"""
    use serde_json::json;
    req.extra.insert("chat_template_kwargs".to_string(), json!({{"enable_thinking": true}}));
    println!("用户: {{}}\\n", prompt);
    println!("AI (Thinking):");

    let resp = client.complete(&req).await?;
    println!("{{}}", resp.content);

    Ok(())
}}
"""
        else:
            main_rs += f"""
    println!("用户: {{}}\\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{{}}", resp.content);

    Ok(())
}}
"""
        with open(os.path.join(root, "main.rs"), "w", encoding="utf-8") as f:
            f.write(main_rs)
            
        test_bat = f"""@echo off
REM {platform} 平台测试 - {feature}
REM 用法: test.bat [api_key] [prompt]

cd /d "%~dp0"

if "%{auth_var_name}%"=="" (
    if "%1"=="" (
        echo Warning: No {auth_var_name} provided.
        set API_KEY=dummy_credential
    ) else (
        set API_KEY=%1
        shift
    )
) else (
    set API_KEY=%{auth_var_name}%
)

if "%1"=="" (
    set PROMPT=你好！请简单介绍一下你自己。
) else (
    set PROMPT=%1
)

echo ========================================
echo   {platform} {feature} Test
echo ========================================
echo.

cargo run --example {platform}_{feature} -- %API_KEY% "%PROMPT%"

echo.
echo ========================================
echo   Test Complete
echo ========================================
"""
        with open(os.path.join(root, "test.bat"), "w", encoding="utf-8") as f:
            f.write(test_bat)
